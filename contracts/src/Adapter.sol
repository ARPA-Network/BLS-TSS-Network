// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {SafeERC20, Address} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {IAdapter} from "./interfaces/IAdapter.sol";
import {IAdapterOwner} from "./interfaces/IAdapterOwner.sol";
import {IController} from "./interfaces/IController.sol";
import {IBasicRandcastConsumerBase} from "./interfaces/IBasicRandcastConsumerBase.sol";
import {AggregatorV3Interface} from "./interfaces/IAggregatorV3.sol";
import {RequestIdBase} from "./utils/RequestIdBase.sol";
import {RandomnessHandler} from "./utils/RandomnessHandler.sol";
import {BLS} from "./libraries/BLS.sol";
// solhint-disable-next-line no-global-import
import "./utils/Utils.sol" as Utils;

contract Adapter is UUPSUpgradeable, IAdapter, IAdapterOwner, RequestIdBase, RandomnessHandler, OwnableUpgradeable {
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
    IERC20 internal _arpa;
    AggregatorV3Interface internal _arpaEthFeed;
    IController internal _controller;

    // Randomness Task State
    uint256 internal _lastAssignedGroupIndex;
    uint256 internal _lastRandomness;
    uint256 internal _randomnessCount;

    AdapterConfig internal _config;
    FeeConfig internal _feeConfig;
    mapping(bytes32 => bytes32) internal _requestCommitments;
    /* consumerAddress - consumer */
    mapping(address => Consumer) internal _consumers;
    /* subId - subscription */
    mapping(uint64 => Subscription) internal _subscriptions;
    uint64 internal _currentSubId;

    // *Structs*
    // Note a nonce of 0 indicates an the consumer is not assigned to that subscription.
    struct Consumer {
        /* subId - nonce */
        mapping(uint64 => uint64) nonces;
        uint64 lastSubscription;
    }

    struct Subscription {
        address owner; // Owner can fund/withdraw/cancel the sub.
        address requestedOwner; // For safely transferring sub ownership.
        address[] consumers;
        uint256 balance; // Token balance used for all consumer requests.
        uint256 inflightCost; // Upper cost for pending requests(except drastic exchange rate changes).
        mapping(bytes32 => uint256) inflightPayments;
        uint64 reqCount; // For fee tiers
        TokenType tokenType; // Every sub is either ARPA or ETH.
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
    event SubscriptionCreated(uint64 indexed subId, address indexed owner, TokenType tokenType);
    event SubscriptionFunded(uint64 indexed subId, uint256 oldBalance, uint256 newBalance);
    event SubscriptionConsumerAdded(uint64 indexed subId, address consumer);
    event RandomnessRequest(
        bytes32 indexed requestId,
        uint64 indexed subId,
        uint256 indexed groupIndex,
        RequestType requestType,
        bytes params,
        address sender,
        uint256 seed,
        uint16 requestConfirmations,
        uint256 callbackGasLimit,
        uint256 callbackMaxGasPrice,
        uint256 estimatedPayment
    );
    event RandomnessRequestResult(
        bytes32 indexed requestId,
        uint256 indexed groupIndex,
        address indexed committer,
        address[] participantMembers,
        uint256 randommness,
        uint256 payment,
        bool success
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
    error InvalidarpaWeiPrice(int256 arpaWei);
    error NoAvailableGroups();
    error NoCorrespondingRequest();
    error IncorrectCommitment();
    error InvalidRequestByEOA();
    error TaskStillExclusive();
    error TaskStillWithinRequestConfirmations();
    error NotFromCommitter();
    error GroupNotExist(uint256 groupIndex);

    // *Modifiers*
    modifier onlySubOwner(uint64 subId) {
        address owner = _subscriptions[subId].owner;
        if (owner == address(0)) {
            revert InvalidSubscription();
        }
        if (msg.sender != owner) {
            revert MustBeSubOwner(owner);
        }
        _;
    }

    modifier nonReentrant() {
        if (_config.reentrancyLock) {
            revert Reentrant();
        }
        _;
    }

    function initialize(address controller, address arpa, address arpaEthFeed) public initializer {
        _controller = IController(controller);
        _arpa = IERC20(arpa);
        _arpaEthFeed = AggregatorV3Interface(arpaEthFeed);

        __Ownable_init();
    }

    // solhint-disable-next-line no-empty-blocks
    function _authorizeUpgrade(address) internal override onlyOwner {}

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
            revert InvalidarpaWeiPrice(fallbackWeiPerUnitArpa);
        }
        _config = AdapterConfig({
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
        _feeConfig = feeConfig;

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
            _feeConfig
        );
    }

    // =============
    // IAdapter
    // =============
    function createSubscription(TokenType tokenType) external override(IAdapter) nonReentrant returns (uint64) {
        _currentSubId++;

        _subscriptions[_currentSubId].owner = msg.sender;
        _subscriptions[_currentSubId].tokenType = tokenType;

        emit SubscriptionCreated(_currentSubId, msg.sender, tokenType);
        return _currentSubId;
    }

    function addConsumer(uint64 subId, address consumer) external override(IAdapter) onlySubOwner(subId) nonReentrant {
        // Already maxed, cannot add any more consumers.
        if (_subscriptions[subId].consumers.length == MAX_CONSUMERS) {
            revert TooManyConsumers();
        }
        if (_consumers[consumer].nonces[subId] != 0) {
            // Idempotence - do nothing if already added.
            // Ensures uniqueness in subscriptions[subId].consumers.
            return;
        }
        // Initialize the nonce to 1, indicating the consumer is allocated.
        _consumers[consumer].nonces[subId] = 1;
        _consumers[consumer].lastSubscription = subId;
        _subscriptions[subId].consumers.push(consumer);

        emit SubscriptionConsumerAdded(subId, consumer);
    }

    function fundSubscription(uint64 subId, uint256 amount) external payable override(IAdapter) nonReentrant {
        if (_subscriptions[subId].owner == address(0)) {
            revert InvalidSubscription();
        }

        if (_subscriptions[subId].tokenType == TokenType.ETH) {
            amount = msg.value;
            (bool sent,) = payable(address(_controller)).call{value: address(this).balance}("");
            require(sent, "Failed to send Ether");
        } else {
            _arpa.safeTransferFrom(msg.sender, address(_controller), amount);
        }

        // We do not check that the msg.sender is the subscription owner,
        // anyone can fund a subscription.
        uint256 oldBalance = _subscriptions[subId].balance;
        _subscriptions[subId].balance += amount;
        emit SubscriptionFunded(subId, oldBalance, oldBalance + amount);
    }

    function requestRandomness(RandomnessRequestParams memory p)
        public
        virtual
        override(IAdapter)
        nonReentrant
        returns (bytes32)
    {
        // solhint-disable-next-line avoid-tx-origin
        if (msg.sender == tx.origin) {
            revert InvalidRequestByEOA();
        }

        // Input validation using the subscription storage.
        if (_subscriptions[p.subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        // Its important to ensure that the consumer is in fact who they say they
        // are, otherwise they could use someone else's subscription balance.
        // A nonce of 0 indicates consumer is not allocated to the sub.
        if (_consumers[msg.sender].nonces[p.subId] == 0) {
            revert InvalidConsumer(p.subId, msg.sender);
        }

        // Choose current available group to handle randomness request(by round robin)
        _lastAssignedGroupIndex = _findGroupToAssignTask();

        // Calculate requestId for the task
        uint256 rawSeed = _makeRandcastInputSeed(p.seed, msg.sender, _consumers[msg.sender].nonces[p.subId]);
        _consumers[msg.sender].nonces[p.subId] += 1;
        bytes32 requestId = _makeRequestId(rawSeed);

        // Estimate upper cost of this fulfillment.
        (uint256 payment, uint256 weiPerUnitArpa) = estimatePaymentAmountInArpa(
            p.callbackGasLimit,
            _config.gasExceptCallback,
            getFeeTier(_subscriptions[p.subId].reqCount + 1),
            p.callbackMaxGasPrice
        );

        if (_subscriptions[p.subId].tokenType == TokenType.ETH) {
            payment = payment * weiPerUnitArpa / 1e18;
        }

        if (_subscriptions[p.subId].balance - _subscriptions[p.subId].inflightCost < payment) {
            revert InsufficientBalanceWhenRequest();
        }
        _subscriptions[p.subId].inflightCost += payment;
        _subscriptions[p.subId].inflightPayments[requestId] = payment;

        _requestCommitments[requestId] = keccak256(
            abi.encode(
                requestId,
                p.subId,
                _lastAssignedGroupIndex,
                p.requestType,
                p.params,
                msg.sender,
                rawSeed,
                p.requestConfirmations,
                p.callbackGasLimit,
                p.callbackMaxGasPrice,
                block.number
            )
        );

        emit RandomnessRequest(
            requestId,
            p.subId,
            _lastAssignedGroupIndex,
            p.requestType,
            p.params,
            msg.sender,
            rawSeed,
            p.requestConfirmations,
            p.callbackGasLimit,
            p.callbackMaxGasPrice,
            payment
        );

        return requestId;
    }

    function fulfillRandomness(
        uint256 groupIndex,
        bytes32 requestId,
        uint256 signature,
        RequestDetail calldata requestDetail,
        PartialSignature[] calldata partialSignatures
    ) public virtual override(IAdapter) nonReentrant {
        uint256 startGas = gasleft();

        bytes32 commitment = _requestCommitments[requestId];
        if (commitment == 0) {
            revert NoCorrespondingRequest();
        }
        if (
            commitment
                != keccak256(
                    abi.encode(
                        requestId,
                        requestDetail.subId,
                        requestDetail.groupIndex,
                        requestDetail.requestType,
                        requestDetail.params,
                        requestDetail.callbackContract,
                        requestDetail.seed,
                        requestDetail.requestConfirmations,
                        requestDetail.callbackGasLimit,
                        requestDetail.callbackMaxGasPrice,
                        requestDetail.blockNum
                    )
                )
        ) {
            revert IncorrectCommitment();
        }

        if (block.number < requestDetail.blockNum + requestDetail.requestConfirmations) {
            revert TaskStillWithinRequestConfirmations();
        }

        if (
            groupIndex != requestDetail.groupIndex
                && block.number <= requestDetail.blockNum + _config.signatureTaskExclusiveWindow
        ) {
            revert TaskStillExclusive();
        }
        if (groupIndex >= _controller.getGroupCount()) {
            revert GroupNotExist(groupIndex);
        }

        address[] memory participantMembers =
            _verifySignature(groupIndex, requestDetail.seed, requestDetail.blockNum, signature, partialSignatures);

        delete _requestCommitments[requestId];

        uint256 randomness = uint256(keccak256(abi.encode(signature)));

        _randomnessCount += 1;
        _lastRandomness = randomness;
        _controller.setLastOutput(randomness);
        // call user fulfill_randomness callback
        bool success = _fulfillCallback(requestId, randomness, requestDetail);
        // Increment the req count for fee tier selection.
        uint64 reqCount = _subscriptions[requestDetail.subId].reqCount;
        _subscriptions[requestDetail.subId].reqCount += 1;

        // We want to charge users exactly for how much gas they use in their callback.
        // The gasAfterPaymentCalculation is meant to cover these additional operations where we
        // decrement the subscription balance and increment the groups withdrawable balance.
        // We also add the flat arpa fee to the payment amount.
        // Its specified in millionths of arpa, if _config.fulfillmentFlatFeeArpaPPM = 1
        // 1 arpa / 1e6 = 1e18 arpa wei / 1e6 = 1e12 arpa wei.
        (uint256 payment, uint256 weiPerUnitArpa) = _calculatePaymentAmountInArpa(
            startGas, _config.gasAfterPaymentCalculation, getFeeTier(reqCount), tx.gasprice
        );

        // rewardRandomness for participants
        _rewardRandomness(participantMembers, payment);

        if (_subscriptions[requestDetail.subId].tokenType == TokenType.ETH) {
            payment = payment * weiPerUnitArpa / 1e18;
        }

        if (_subscriptions[requestDetail.subId].balance < payment) {
            revert InsufficientBalanceWhenFulfill();
        }
        _subscriptions[requestDetail.subId].inflightCost -=
            _subscriptions[requestDetail.subId].inflightPayments[requestId];
        delete _subscriptions[requestDetail.subId].inflightPayments[requestId];
        _subscriptions[requestDetail.subId].balance -= payment;

        // Include payment in the event for tracking costs.
        emit RandomnessRequestResult(
            requestId, groupIndex, msg.sender, participantMembers, randomness, payment, success
        );
    }

    function getLastSubscription(address consumer) public view override(IAdapter) returns (uint64) {
        return _consumers[consumer].lastSubscription;
    }

    function getSubscription(uint64 subId)
        public
        view
        override(IAdapter)
        returns (uint256 balance, uint256 inflightCost, uint64 reqCount, address owner, address[] memory consumers)
    {
        if (_subscriptions[subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        return (
            _subscriptions[subId].balance,
            _subscriptions[subId].inflightCost,
            _subscriptions[subId].reqCount,
            _subscriptions[subId].owner,
            _subscriptions[subId].consumers
        );
    }

    function getPendingRequestCommitment(bytes32 requestId) public view override(IAdapter) returns (bytes32) {
        return _requestCommitments[requestId];
    }

    function getLastRandomness() external view override(IAdapter) returns (uint256) {
        return _lastRandomness;
    }

    function getRandomnessCount() external view override(IAdapter) returns (uint256) {
        return _randomnessCount;
    }

    function getFeeTier(uint64 reqCount) public view override(IAdapter) returns (uint32) {
        FeeConfig memory fc = _feeConfig;
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

    function estimatePaymentAmountInArpa(
        uint256 callbackGasLimit,
        uint256 gasExceptCallback,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) public view override(IAdapter) returns (uint256, uint256) {
        int256 weiPerUnitArpa;
        weiPerUnitArpa = _getArpaEthFeedData();
        if (weiPerUnitArpa <= 0) {
            revert InvalidarpaWeiPrice(weiPerUnitArpa);
        }
        // (1e18) (wei/gas * gas) / (wei/arpa) = arpa wei
        uint256 paymentNoFee = 1e18 * weiPerUnitGas * (gasExceptCallback + callbackGasLimit) / uint256(weiPerUnitArpa);
        uint256 fee = 1e12 * uint256(fulfillmentFlatFeeArpaPPM);

        return (paymentNoFee + fee, uint256(weiPerUnitArpa));
    }

    // =============
    // Internal
    // =============

    function _findGroupToAssignTask() internal view returns (uint256) {
        uint256[] memory validGroupIndices = _controller.getValidGroupIndices();

        if (validGroupIndices.length == 0) {
            revert NoAvailableGroups();
        }

        uint256 groupCount = _controller.getGroupCount();

        uint256 currentAssignedGroupIndex = (_lastAssignedGroupIndex + 1) % groupCount;

        while (!Utils.containElement(validGroupIndices, currentAssignedGroupIndex)) {
            currentAssignedGroupIndex = (currentAssignedGroupIndex + 1) % groupCount;
        }

        return currentAssignedGroupIndex;
    }

    function _rewardRandomness(address[] memory participantMembers, uint256 payment) internal {
        address[] memory committer = new address[](1);
        committer[0] = msg.sender;
        _controller.addReward(committer, _config.committerRewardPerSignature);
        _controller.addReward(participantMembers, _config.rewardPerSignature + payment / participantMembers.length);
    }

    function _verifySignature(
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

        IController.Group memory g = _controller.getGroup(groupIndex);

        if (!Utils.containElement(g.committers, msg.sender)) {
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

    function _fulfillCallback(bytes32 requestId, uint256 randomness, RequestDetail memory requestDetail)
        internal
        returns (bool success)
    {
        IBasicRandcastConsumerBase b;
        bytes memory resp;
        if (requestDetail.requestType == RequestType.Randomness) {
            resp = abi.encodeWithSelector(b.rawFulfillRandomness.selector, requestId, randomness);
        } else if (requestDetail.requestType == RequestType.RandomWords) {
            uint32 numWords = abi.decode(requestDetail.params, (uint32));
            uint256[] memory randomWords = new uint256[](numWords);
            for (uint256 i = 0; i < numWords; i++) {
                randomWords[i] = uint256(keccak256(abi.encode(randomness, i)));
            }
            resp = abi.encodeWithSelector(b.rawFulfillRandomWords.selector, requestId, randomWords);
        } else if (requestDetail.requestType == RequestType.Shuffling) {
            uint32 upper = abi.decode(requestDetail.params, (uint32));
            uint256[] memory shuffledArray = _shuffle(upper, randomness);
            resp = abi.encodeWithSelector(b.rawFulfillShuffledArray.selector, requestId, shuffledArray);
        }

        // Call with explicitly the amount of callback gas requested
        // Important to not let them exhaust the gas budget and avoid oracle payment.
        // Do not allow any non-view/non-pure coordinator functions to be called
        // during the consumers callback code via reentrancyLock.
        // Note that callWithExactGas will revert if we do not have sufficient gas
        // to give the callee their requested amount.
        _config.reentrancyLock = true;
        success = Utils.callWithExactGas(requestDetail.callbackGasLimit, requestDetail.callbackContract, resp);
        _config.reentrancyLock = false;
    }

    // Get the amount of gas used for fulfillment
    function _calculatePaymentAmountInArpa(
        uint256 startGas,
        uint256 gasAfterPaymentCalculation,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) internal view returns (uint256, uint256) {
        int256 weiPerUnitArpa;
        weiPerUnitArpa = _getArpaEthFeedData();
        if (weiPerUnitArpa <= 0) {
            revert InvalidarpaWeiPrice(weiPerUnitArpa);
        }

        // (1e18) (wei/gas * gas) / (wei/arpa) = arpa wei
        uint256 paymentNoFee =
            1e18 * (weiPerUnitGas * (gasAfterPaymentCalculation + startGas - gasleft())) / uint256(weiPerUnitArpa);
        uint256 fee = 1e12 * uint256(fulfillmentFlatFeeArpaPPM);
        if (paymentNoFee > (15e26 - fee)) {
            revert PaymentTooLarge(); // Payment + fee cannot be more than all of the arpa in existence.
        }
        return (paymentNoFee + fee, uint256(weiPerUnitArpa));
    }

    function _getArpaEthFeedData() private view returns (int256) {
        uint32 stalenessSeconds = _config.stalenessSeconds;
        bool staleFallback = stalenessSeconds > 0;
        uint256 timestamp;
        int256 weiPerUnitArpa;
        (, weiPerUnitArpa,, timestamp,) = _arpaEthFeed.latestRoundData();
        // solhint-disable-next-line not-rely-on-time
        if (staleFallback && stalenessSeconds < block.timestamp - timestamp) {
            weiPerUnitArpa = _config.fallbackWeiPerUnitArpa;
        }
        return weiPerUnitArpa;
    }
}
