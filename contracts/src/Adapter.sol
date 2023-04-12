// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

import "./interfaces/IAdapter.sol";
import "./interfaces/IAdapterOwner.sol";
import "./interfaces/IController.sol";
import "./interfaces/IBasicRandcastConsumerBase.sol";
import "./interfaces/IAggregatorV3.sol";
import "./utils/Utils.sol";
import "./utils/RequestIdBase.sol";
import "./utils/RandomnessHandler.sol";
import {BLS} from "./libraries/BLS.sol";

contract Adapter is IAdapter, IAdapterOwner, RequestIdBase, RandomnessHandler, Ownable {
    using SafeERC20 for IERC20;
    using Address for address;

    // *Constants*
    // We need to maintain a list of consuming addresses.
    // This bound ensures we are able to loop over them as needed.
    // Should a user require more consumers, they can use multiple subscriptions.
    uint16 public constant MAX_CONSUMERS = 100;
    // TODO Set this maximum to 200 to give us a 56 block window to fulfill
    // the request before requiring the block hash feeder.
    uint16 public constant MAX_REQUEST_CONFIRMATIONS = 200;

    // *State Variables*
    IERC20 private immutable i_ARPA;
    AggregatorV3Interface private immutable i_ARPA_ETH_Feed;
    IController private s_controller;

    // Randomness Task State
    uint256 private s_lastAssignedGroupIndex;
    uint256 private s_lastRandomness;

    AdapterConfig private s_config;
    FeeConfig private s_feeConfig;
    mapping(bytes32 => Callback) private s_callbacks;
    mapping(address => Consumer) /* consumerAddress */ /* consumer */ private s_consumers;
    mapping(uint64 => Subscription) /* subId */ /* subscription */ private s_subscriptions;
    uint64 private s_currentSubId;

    // *Structs*
    // Note a nonce of 0 indicates an the consumer is not assigned to that subscription.
    struct Consumer {
        mapping(uint64 => uint64) nonces; /* subId */ /* nonce */
        uint64 lastSubscription;
    }

    struct Subscription {
        address owner; // Owner can fund/withdraw/cancel the sub.
        address requestedOwner; // For safely transferring sub ownership.
        address[] consumers;
        uint96 balance; // Arpa balance used for all consumer requests.
        uint96 inflightCost; // Arpa upper cost for pending requests.
        mapping(bytes32 => uint96) inflightPayments;
        uint64 reqCount; // For fee tiers
    }

    // *Events*
    event AdapterConfigSet(
        uint16 minimumRequestConfirmations,
        uint32 maxGasLimit,
        uint32 stalenessSeconds,
        uint32 gasAfterPaymentCalculation,
        uint32 gasExceptCallback,
        int256 fallbackWeiPerUnitArpa,
        uint256 signatureTaskExclusiveWindow,
        uint256 rewardPerSignature,
        uint256 committerRewardPerSignature,
        FeeConfig feeConfig
    );
    event SubscriptionCreated(uint64 indexed subId, address owner);
    event SubscriptionFunded(uint64 indexed subId, uint256 oldBalance, uint256 newBalance);
    event SubscriptionConsumerAdded(uint64 indexed subId, address consumer);
    event RandomnessRequest(
        bytes32 indexed requestId,
        uint64 indexed subId,
        uint256 indexed groupIndex,
        address sender,
        uint256 seed,
        uint16 requestConfirmations,
        uint256 callbackGasLimit,
        uint256 callbackMaxGasPrice
    );
    event RandomnessRequestResult(
        bytes32 indexed requestId, uint256 indexed groupIndex, uint256 randommness, uint256 payment, bool success
    );

    // *Errors*
    error Reentrant();
    error InvalidRequestConfirmations(uint16 have, uint16 min, uint16 max);
    error TooManyConsumers();
    error InsufficientBalanceWhenRequest();
    error InsufficientBalanceWhenFulfill();
    error InvalidConsumer(uint64 subId, address consumer);
    error InvalidSubscription();
    error MustBeSubOwner(address owner);
    error PaymentTooLarge();
    error InvalidArpaWeiPrice(int256 arpaWei);
    error NoAvailableGroups();
    error NoCorrespondingRequest();
    error InvalidRequestByEOA();
    error TaskStillExclusive();
    error TaskStillWithinRequestConfirmations();
    error NotFromCommitter();
    error GroupNotExist(uint256 groupIndex);

    // *Modifiers*
    modifier onlySubOwner(uint64 subId) {
        address owner = s_subscriptions[subId].owner;
        if (owner == address(0)) {
            revert InvalidSubscription();
        }
        if (msg.sender != owner) {
            revert MustBeSubOwner(owner);
        }
        _;
    }

    modifier nonReentrant() {
        if (s_config.reentrancyLock) {
            revert Reentrant();
        }
        _;
    }

    constructor(address controller, address arpa, address arpaEthFeed) {
        s_controller = IController(controller);
        i_ARPA = IERC20(arpa);
        i_ARPA_ETH_Feed = AggregatorV3Interface(arpaEthFeed);
    }

    // =============
    // IAdapterOwner
    // =============
    function setAdapterConfig(
        uint16 minimumRequestConfirmations,
        uint32 maxGasLimit,
        uint32 stalenessSeconds,
        uint32 gasAfterPaymentCalculation,
        uint32 gasExceptCallback,
        int256 fallbackWeiPerUnitArpa,
        uint256 signatureTaskExclusiveWindow,
        uint256 rewardPerSignature,
        uint256 committerRewardPerSignature,
        FeeConfig memory feeConfig
    ) external override(IAdapterOwner) onlyOwner {
        if (minimumRequestConfirmations > MAX_REQUEST_CONFIRMATIONS) {
            revert InvalidRequestConfirmations(
                minimumRequestConfirmations, minimumRequestConfirmations, MAX_REQUEST_CONFIRMATIONS
            );
        }
        if (fallbackWeiPerUnitArpa <= 0) {
            revert InvalidArpaWeiPrice(fallbackWeiPerUnitArpa);
        }
        s_config = AdapterConfig({
            minimumRequestConfirmations: minimumRequestConfirmations,
            maxGasLimit: maxGasLimit,
            stalenessSeconds: stalenessSeconds,
            fallbackWeiPerUnitArpa: fallbackWeiPerUnitArpa,
            gasAfterPaymentCalculation: gasAfterPaymentCalculation,
            gasExceptCallback: gasExceptCallback,
            signatureTaskExclusiveWindow: signatureTaskExclusiveWindow,
            rewardPerSignature: rewardPerSignature,
            committerRewardPerSignature: committerRewardPerSignature,
            reentrancyLock: false
        });
        s_feeConfig = feeConfig;

        emit AdapterConfigSet(
            minimumRequestConfirmations,
            maxGasLimit,
            stalenessSeconds,
            gasAfterPaymentCalculation,
            gasExceptCallback,
            fallbackWeiPerUnitArpa,
            signatureTaskExclusiveWindow,
            rewardPerSignature,
            committerRewardPerSignature,
            s_feeConfig
        );
    }

    // =============
    // IAdapter
    // =============
    function createSubscription() external override(IAdapter) nonReentrant returns (uint64) {
        s_currentSubId++;

        s_subscriptions[s_currentSubId].owner = msg.sender;

        emit SubscriptionCreated(s_currentSubId, msg.sender);
        return s_currentSubId;
    }

    function addConsumer(uint64 subId, address consumer) external override(IAdapter) onlySubOwner(subId) nonReentrant {
        // Already maxed, cannot add any more consumers.
        if (s_subscriptions[subId].consumers.length == MAX_CONSUMERS) {
            revert TooManyConsumers();
        }
        if (s_consumers[consumer].nonces[subId] != 0) {
            // Idempotence - do nothing if already added.
            // Ensures uniqueness in subscriptions[subId].consumers.
            return;
        }
        // Initialize the nonce to 1, indicating the consumer is allocated.
        s_consumers[consumer].nonces[subId] = 1;
        s_consumers[consumer].lastSubscription = subId;
        s_subscriptions[subId].consumers.push(consumer);

        emit SubscriptionConsumerAdded(subId, consumer);
    }

    function fundSubscription(uint64 subId, uint256 amount) external override(IAdapter) nonReentrant {
        if (s_subscriptions[subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        i_ARPA.safeTransferFrom(msg.sender, address(s_controller), amount);
        // We do not check that the msg.sender is the subscription owner,
        // anyone can fund a subscription.
        uint256 oldBalance = s_subscriptions[subId].balance;
        s_subscriptions[subId].balance += uint96(amount);
        emit SubscriptionFunded(subId, oldBalance, oldBalance + amount);
    }

    function requestRandomness(RandomnessRequestParams memory p) external override(IAdapter) returns (bytes32) {
        if (msg.sender == tx.origin) {
            revert InvalidRequestByEOA();
        }

        // Input validation using the subscription storage.
        if (s_subscriptions[p.subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        // Its important to ensure that the consumer is in fact who they say they
        // are, otherwise they could use someone else's subscription balance.
        // A nonce of 0 indicates consumer is not allocated to the sub.
        uint64 currentNonce = s_consumers[msg.sender].nonces[p.subId];
        if (currentNonce == 0) {
            revert InvalidConsumer(p.subId, msg.sender);
        }

        // Choose current available group to handle randomness request(by round robin)
        uint256 currentAssignedGroupIndex = findGroupToAssignTask();

        // Update global state
        s_lastAssignedGroupIndex = currentAssignedGroupIndex;

        // Calculate requestId for the task
        uint64 nonce = s_consumers[msg.sender].nonces[p.subId];
        uint256 rawSeed = makeRandcastInputSeed(p.seed, msg.sender, nonce);
        s_consumers[msg.sender].nonces[p.subId] += 1;
        bytes32 requestId = makeRequestId(rawSeed);

        // Estimate upper cost of this fulfillment.
        uint64 reqCount = s_subscriptions[p.subId].reqCount;
        uint96 payment = estimatePaymentAmount(
            p.callbackGasLimit, s_config.gasExceptCallback, getFeeTier(reqCount + 1), p.callbackMaxGasPrice
        );

        if (s_subscriptions[p.subId].balance - s_subscriptions[p.subId].inflightCost < payment) {
            revert InsufficientBalanceWhenRequest();
        }
        s_subscriptions[p.subId].inflightCost += payment;
        s_subscriptions[p.subId].inflightPayments[requestId] = payment;

        // Record callback struct
        assert(s_callbacks[requestId].callbackContract == address(0));
        Callback storage callback = s_callbacks[requestId];
        callback.callbackContract = msg.sender;
        callback.requestType = p.requestType;
        callback.params = p.params;
        callback.subId = p.subId;
        callback.seed = rawSeed;
        callback.groupIndex = currentAssignedGroupIndex;
        callback.blockNum = block.number;
        callback.requestConfirmations = p.requestConfirmations;
        callback.callbackGasLimit = p.callbackGasLimit;
        callback.callbackMaxGasPrice = p.callbackMaxGasPrice;

        emit RandomnessRequest(
            requestId,
            callback.subId,
            currentAssignedGroupIndex,
            msg.sender,
            callback.seed,
            callback.requestConfirmations,
            callback.callbackGasLimit,
            callback.callbackMaxGasPrice
        );

        return requestId;
    }

    function fulfillRandomness(
        uint256 groupIndex,
        bytes32 requestId,
        uint256 signature,
        PartialSignature[] memory partialSignatures
    ) public override(IAdapter) {
        uint256 startGas = gasleft();

        Callback memory callback = s_callbacks[requestId];

        if (callback.seed == 0) {
            revert NoCorrespondingRequest();
        }

        if (block.number < callback.blockNum + callback.requestConfirmations) {
            revert TaskStillWithinRequestConfirmations();
        }

        if (
            groupIndex != callback.groupIndex
                && block.number <= callback.blockNum + s_config.signatureTaskExclusiveWindow
        ) {
            revert TaskStillExclusive();
        }
        if (groupIndex >= s_controller.getGroupCount()) {
            revert GroupNotExist(groupIndex);
        }

        address[] memory participantMembers =
            verifySignature(groupIndex, callback.seed, callback.blockNum, signature, partialSignatures);

        uint256 randomness = uint256(keccak256(abi.encode(signature)));

        s_lastRandomness = randomness;
        s_controller.setLastOutput(randomness);
        // call user fulfill_randomness callback
        bool success = fulfillCallback(requestId, randomness, callback);
        // Increment the req count for fee tier selection.
        uint64 reqCount = s_subscriptions[callback.subId].reqCount;
        s_subscriptions[callback.subId].reqCount += 1;

        // We want to charge users exactly for how much gas they use in their callback.
        // The gasAfterPaymentCalculation is meant to cover these additional operations where we
        // decrement the subscription balance and increment the groups withdrawable balance.
        // We also add the flat arpa fee to the payment amount.
        // Its specified in millionths of arpa, if s_config.fulfillmentFlatFeeArpaPPM = 1
        // 1 arpa / 1e6 = 1e18 arpa wei / 1e6 = 1e12 arpa wei.
        uint96 payment =
            calculatePaymentAmount(startGas, s_config.gasAfterPaymentCalculation, getFeeTier(reqCount), tx.gasprice);

        if (s_subscriptions[callback.subId].balance < payment) {
            revert InsufficientBalanceWhenFulfill();
        }
        s_subscriptions[callback.subId].inflightCost -= s_subscriptions[callback.subId].inflightPayments[requestId];
        delete s_subscriptions[callback.subId].inflightPayments[requestId];
        s_subscriptions[callback.subId].balance -= payment;

        // rewardRandomness for participants
        rewardRandomness(participantMembers, payment);

        // Include payment in the event for tracking costs.
        emit RandomnessRequestResult(requestId, groupIndex, randomness, payment, success);
    }

    function getLastSubscription(address consumer) public view override(IAdapter) returns (uint64) {
        return s_consumers[consumer].lastSubscription;
    }

    function getSubscription(uint64 subId)
        public
        view
        override(IAdapter)
        returns (uint96 balance, uint96 inflightCost, uint64 reqCount, address owner, address[] memory consumers)
    {
        if (s_subscriptions[subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        return (
            s_subscriptions[subId].balance,
            s_subscriptions[subId].inflightCost,
            s_subscriptions[subId].reqCount,
            s_subscriptions[subId].owner,
            s_subscriptions[subId].consumers
        );
    }

    function getPendingRequest(bytes32 requestId) public view override(IAdapter) returns (Callback memory) {
        return s_callbacks[requestId];
    }

    function getLastRandomness() external view override(IAdapter) returns (uint256) {
        return s_lastRandomness;
    }

    function getFeeTier(uint64 reqCount) public view override(IAdapter) returns (uint32) {
        FeeConfig memory fc = s_feeConfig;
        if (0 <= reqCount && reqCount <= fc.reqsForTier2) {
            return fc.fulfillmentFlatFeeArpaPPMTier1;
        }
        if (fc.reqsForTier2 < reqCount && reqCount <= fc.reqsForTier3) {
            return fc.fulfillmentFlatFeeArpaPPMTier2;
        }
        if (fc.reqsForTier3 < reqCount && reqCount <= fc.reqsForTier4) {
            return fc.fulfillmentFlatFeeArpaPPMTier3;
        }
        if (fc.reqsForTier4 < reqCount && reqCount <= fc.reqsForTier5) {
            return fc.fulfillmentFlatFeeArpaPPMTier4;
        }
        return fc.fulfillmentFlatFeeArpaPPMTier5;
    }

    function estimatePaymentAmount(
        uint256 callbackGasLimit,
        uint256 gasExceptCallback,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) public view override(IAdapter) returns (uint96) {
        int256 weiPerUnitArpa;
        weiPerUnitArpa = getFeedData();
        if (weiPerUnitArpa <= 0) {
            revert InvalidArpaWeiPrice(weiPerUnitArpa);
        }
        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        uint256 paymentNoFee = (1e18 * weiPerUnitGas * (gasExceptCallback + callbackGasLimit)) / uint256(weiPerUnitArpa);
        uint256 fee = 1e12 * uint256(fulfillmentFlatFeeArpaPPM);
        return uint96(paymentNoFee + fee);
    }

    // =============
    // Internal
    // =============

    function findGroupToAssignTask() internal view returns (uint256) {
        uint256[] memory validGroupIndices = s_controller.getValidGroupIndices();

        if (validGroupIndices.length == 0) {
            revert NoAvailableGroups();
        }

        uint256 groupCount = s_controller.getGroupCount();

        uint256 currentAssignedGroupIndex = (s_lastAssignedGroupIndex + 1) % groupCount;

        while (!containElement(validGroupIndices, currentAssignedGroupIndex)) {
            currentAssignedGroupIndex = (currentAssignedGroupIndex + 1) % groupCount;
        }

        return currentAssignedGroupIndex;
    }

    function rewardRandomness(address[] memory participantMembers, uint96 payment) internal {
        s_controller.addReward(msg.sender, s_config.committerRewardPerSignature);
        for (uint256 i = 0; i < participantMembers.length; i++) {
            s_controller.addReward(
                participantMembers[i], s_config.rewardPerSignature + payment / participantMembers.length
            );
        }
    }

    function verifySignature(
        uint256 groupIndex,
        uint256 seed,
        uint256 blockNum,
        uint256 signature,
        PartialSignature[] memory partialSignatures
    ) internal view returns (address[] memory participantMembers) {
        if (!BLS.isValid(signature)) {
            revert BLS.InvalidSignatureFormat();
        }

        if (partialSignatures.length == 0) {
            revert BLS.EmptyPartialSignatures();
        }

        IController.Group memory g = s_controller.getGroup(groupIndex);

        if (!containElement(g.committers, msg.sender)) {
            revert NotFromCommitter();
        }

        bytes memory actualSeed = abi.encodePacked(seed, blockNum);

        uint256[2] memory message = BLS.hashToPoint(actualSeed);

        // verify tss-aggregation signature for randomness
        if (!BLS.verifySingle(BLS.decompress(signature), g.publicKey, message)) {
            revert BLS.InvalidSignature();
        }

        // verify bls-aggregation signature for incentivizing worker list
        uint256[2][] memory partials = new uint256[2][](partialSignatures.length);
        uint256[4][] memory pubkeys = new uint256[4][](partialSignatures.length);
        participantMembers = new address[](partialSignatures.length);
        for (uint256 i = 0; i < partialSignatures.length; i++) {
            if (!BLS.isValid(partialSignatures[i].partialSignature)) {
                revert BLS.InvalidPartialSignatureFormat();
            }
            partials[i] = BLS.decompress(partialSignatures[i].partialSignature);
            pubkeys[i] = g.members[partialSignatures[i].index].partialPublicKey;
            participantMembers[i] = g.members[partialSignatures[i].index].nodeIdAddress;
        }
        if (!BLS.verifyPartials(partials, pubkeys, message)) {
            revert BLS.InvalidPartialSignatures();
        }
    }

    function fulfillCallback(bytes32 requestId, uint256 randomness, Callback memory callback)
        internal
        returns (bool success)
    {
        IBasicRandcastConsumerBase b;
        bytes memory resp;
        if (callback.requestType == RequestType.Randomness) {
            resp = abi.encodeWithSelector(b.rawFulfillRandomness.selector, requestId, randomness);
        } else if (callback.requestType == RequestType.RandomWords) {
            uint32 numWords = abi.decode(callback.params, (uint32));
            uint256[] memory randomWords = new uint256[](numWords);
            for (uint256 i = 0; i < numWords; i++) {
                randomWords[i] = uint256(keccak256(abi.encode(randomness, i)));
            }
            resp = abi.encodeWithSelector(b.rawFulfillRandomWords.selector, requestId, randomWords);
        } else if (callback.requestType == RequestType.Shuffling) {
            uint32 upper = abi.decode(callback.params, (uint32));
            uint256[] memory shuffledArray = shuffle(upper, randomness);
            resp = abi.encodeWithSelector(b.rawFulfillShuffledArray.selector, requestId, shuffledArray);
        }

        delete s_callbacks[requestId];

        // Call with explicitly the amount of callback gas requested
        // Important to not let them exhaust the gas budget and avoid oracle payment.
        // Do not allow any non-view/non-pure coordinator functions to be called
        // during the consumers callback code via reentrancyLock.
        // Note that callWithExactGas will revert if we do not have sufficient gas
        // to give the callee their requested amount.
        s_config.reentrancyLock = true;
        success = callWithExactGas(callback.callbackGasLimit, callback.callbackContract, resp);
        s_config.reentrancyLock = false;
    }

    // Get the amount of gas used for fulfillment
    function calculatePaymentAmount(
        uint256 startGas,
        uint256 gasAfterPaymentCalculation,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) internal view returns (uint96) {
        int256 weiPerUnitArpa;
        weiPerUnitArpa = getFeedData();
        if (weiPerUnitArpa <= 0) {
            revert InvalidArpaWeiPrice(weiPerUnitArpa);
        }
        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        uint256 paymentNoFee =
            (1e18 * weiPerUnitGas * (gasAfterPaymentCalculation + startGas - gasleft())) / uint256(weiPerUnitArpa);
        uint256 fee = 1e12 * uint256(fulfillmentFlatFeeArpaPPM);
        if (paymentNoFee > (15e26 - fee)) {
            revert PaymentTooLarge(); // Payment + fee cannot be more than all of the arpa in existence.
        }
        return uint96(paymentNoFee + fee);
    }

    function getFeedData() private view returns (int256) {
        uint32 stalenessSeconds = s_config.stalenessSeconds;
        bool staleFallback = stalenessSeconds > 0;
        uint256 timestamp;
        int256 weiPerUnitArpa;
        (, weiPerUnitArpa,, timestamp,) = i_ARPA_ETH_Feed.latestRoundData();
        // solhint-disable-next-line not-rely-on-time
        if (staleFallback && stalenessSeconds < block.timestamp - timestamp) {
            weiPerUnitArpa = s_config.fallbackWeiPerUnitArpa;
        }
        return weiPerUnitArpa;
    }
}
