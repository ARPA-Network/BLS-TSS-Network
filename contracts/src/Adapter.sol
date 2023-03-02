// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

import "./interfaces/IAdapter.sol";
import "./interfaces/IBasicRandcastConsumerBase.sol";
import "./interfaces/IAggregatorV3.sol";
import "./utils/RequestIdBase.sol";
import "./utils/RandomnessHandler.sol";
import "./libraries/BLS.sol";

contract Adapter is IAdapter, RequestIdBase, RandomnessHandler, Ownable {
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
    // 5k is plenty for an EXTCODESIZE call (2600) + warm CALL (100)
    // and some arithmetic operations.
    uint256 private constant GAS_FOR_CALL_EXACT_CHECK = 5_000;

    // *State Variables*
    IERC20 public immutable ARPA;
    AggregatorV3Interface public immutable ARPA_ETH_FEED;
    // Group State
    uint256 epoch;
    uint256 public groupCount; // Number of groups
    mapping(uint256 => Group) public groups; // group_index => Group struct
    mapping(address => uint256) public rewards; // maps node address to reward amount

    // Randomness Task State
    uint256 public lastAssignedGroupIndex;
    // TODO initialize this value
    uint256 public lastOutput = 0x2222222222222222; // global last output

    int256 private s_fallbackWeiPerUnitArpa;
    AdapterConfig private s_config;
    FeeConfig private s_feeConfig;
    mapping(bytes32 => Callback) public s_callbacks;
    mapping(address => Consumer) /* consumerAddress */ /* consumer */ private s_consumers;
    mapping(uint64 => Subscription) /* subId */ /* subscription */ private s_subscriptions;
    uint64 private s_currentSubId;
    mapping(uint256 => uint96) /* group */ /* ARPA balance */ private s_withdrawableTokens;

    // *Structs*
    struct Group {
        uint256 index; // group_index
        uint256 epoch; // 0
        uint256 size; // 0
        uint256 threshold; // DEFAULT_MINIMUM_THRESHOLD
        Member[] members; // Map in rust mock contract
        address[] committers;
        CommitCache[] commitCacheList; // Map in rust mock contract
        bool isStrictlyMajorityConsensusReached;
        uint256[4] publicKey;
    }

    struct Member {
        address nodeIdAddress;
        uint256[4] partialPublicKey;
    }

    struct CommitResult {
        uint256 groupEpoch;
        uint256[4] publicKey;
        address[] disqualifiedNodes;
    }

    struct CommitCache {
        address[] nodeIdAddress;
        CommitResult commitResult;
    }

    struct AdapterConfig {
        // Minimum number of blocks a request must wait before being fulfilled.
        uint16 minimumRequestConfirmations;
        // Maximum gas limit for fulfillRandomness requests.
        uint32 maxGasLimit;
        // Reentrancy protection.
        bool reentrancyLock;
        // stalenessSeconds is how long before we consider the feed price to be stale
        // and fallback to fallbackWeiPerUnitArpa.
        uint32 stalenessSeconds;
        // Gas to cover group payment after we calculate the payment.
        // We make it configurable in case those operations are repriced.
        uint32 gasAfterPaymentCalculation;
        // Gas except callback during fulfillment of randomness. Only used for estimating inflight cost.
        uint32 gasExceptCallback;
        // The assigned group is exclusive for fulfilling the task within this block window
        uint256 signatureTaskExclusiveWindow;
        // reward per signature for every participating node
        uint256 rewardPerSignature;
        // reward per signature for the committer
        uint256 committerRewardPerSignature;
    }

    struct FeeConfig {
        // Flat fee charged per fulfillment in millionths of arpa
        uint32 fulfillmentFlatFeeArpaPPMTier1;
        uint32 fulfillmentFlatFeeArpaPPMTier2;
        uint32 fulfillmentFlatFeeArpaPPMTier3;
        uint32 fulfillmentFlatFeeArpaPPMTier4;
        uint32 fulfillmentFlatFeeArpaPPMTier5;
        uint24 reqsForTier2;
        uint24 reqsForTier3;
        uint24 reqsForTier4;
        uint24 reqsForTier5;
    }

    // TODO only record the hash of the callback params to save storage gas
    struct Callback {
        address callbackContract;
        RequestType requestType;
        bytes params;
        uint64 subId;
        uint256 seed;
        uint256 groupIndex;
        uint256 blockNum;
        uint16 requestConfirmations;
        uint256 callbackGasLimit;
        uint256 callbackMaxGasPrice;
    }

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
    error InvalidRequestByEOA(string message);
    error TaskStillExclusive();
    error NotFromCommitter();
    error InvalidSignatureFormat();
    error InvalidSignature();
    error InvalidPartialSignatureFormat();
    error InvalidPartialSignatures();
    error EmptyPartialSignatures();
    error InvalidPublicKey();
    error InvalidPartialPublicKey();

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

    constructor(address arpa, address arpaEthFeed) {
        ARPA = IERC20(arpa);
        ARPA_ETH_FEED = AggregatorV3Interface(arpaEthFeed);
    }

    function createSubscription() external nonReentrant returns (uint64) {
        s_currentSubId++;

        s_subscriptions[s_currentSubId].owner = msg.sender;

        emit SubscriptionCreated(s_currentSubId, msg.sender);
        return s_currentSubId;
    }

    function addConsumer(uint64 subId, address consumer) external onlySubOwner(subId) nonReentrant {
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

    function fundSubscription(uint64 subId, uint256 amount) external nonReentrant {
        if (s_subscriptions[subId].owner == address(0)) {
            revert InvalidSubscription();
        }
        ARPA.safeTransferFrom(msg.sender, address(this), amount);
        // We do not check that the msg.sender is the subscription owner,
        // anyone can fund a subscription.
        uint256 oldBalance = s_subscriptions[subId].balance;
        s_subscriptions[subId].balance += uint96(amount);
        emit SubscriptionFunded(subId, oldBalance, oldBalance + amount);
    }

    // Get list of all group indexes where group.isStrictlyMajorityConsensusReached == true
    function validGroupIndices() public view returns (uint256[] memory) {
        uint256[] memory groupIndices = new uint256[](groupCount); //max length is group count
        uint256 index = 0;
        for (uint256 i = 0; i < groupCount; i++) {
            Group memory g = groups[i];
            if (g.isStrictlyMajorityConsensusReached) {
                groupIndices[index] = i;
                index++;
            }
        }

        // create result array of correct size (remove possible trailing zero elements)
        uint256[] memory result = new uint256[](index);
        for (uint256 i = 0; i < index; i++) {
            result[i] = groupIndices[i];
        }

        return result;
    }

    function containElement(uint256[] memory arr, uint256 element) internal pure returns (bool) {
        for (uint256 i = 0; i < arr.length; i++) {
            if (arr[i] == element) {
                return true;
            }
        }
        return false;
    }

    function containElement(address[] memory arr, address element) internal pure returns (bool) {
        for (uint256 i = 0; i < arr.length; i++) {
            if (arr[i] == element) {
                return true;
            }
        }
        return false;
    }

    function findGroupToAssignTask() internal view returns (uint256) {
        uint256[] memory _validGroupIndices = validGroupIndices();

        if (_validGroupIndices.length == 0) {
            revert NoAvailableGroups();
        }

        uint256 currentAssignedGroupIndex = (lastAssignedGroupIndex + 1) % groupCount;

        while (!containElement(_validGroupIndices, currentAssignedGroupIndex)) {
            currentAssignedGroupIndex = (currentAssignedGroupIndex + 1) % groupCount;
        }

        return currentAssignedGroupIndex;
    }

    function requestRandomness(RandomnessRequestParams memory p) external override returns (bytes32) {
        if (msg.sender == tx.origin) {
            revert InvalidRequestByEOA(
                "Please request by extending GeneralRandcastConsumerBase so that we can callback with randomness."
            );
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
        lastAssignedGroupIndex = currentAssignedGroupIndex;

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
            currentAssignedGroupIndex,
            requestId,
            msg.sender,
            callback.subId,
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
    ) public override {
        uint256 startGas = gasleft();

        Callback memory callback = s_callbacks[requestId];

        if (callback.seed == 0) {
            revert NoCorrespondingRequest();
        }

        require(block.number >= callback.blockNum + callback.requestConfirmations, "Too early to fulfill");

        if (
            groupIndex != callback.groupIndex
                && block.number <= callback.blockNum + s_config.signatureTaskExclusiveWindow
        ) {
            revert TaskStillExclusive();
        }

        require(groupIndex < groupCount, "Group does not exist");

        if (!BLS.isValid(signature)) {
            revert InvalidSignatureFormat();
        }

        if (partialSignatures.length == 0) {
            revert EmptyPartialSignatures();
        }

        Group storage g = groups[groupIndex];

        if (!containElement(g.committers, msg.sender)) {
            revert NotFromCommitter();
        }

        bytes memory actualSeed = abi.encodePacked(callback.seed, callback.blockNum);

        uint256[2] memory message = BLS.hashToPoint(actualSeed);

        // verify tss-aggregation signature for randomness
        if (!BLS.verifySingle(BLS.decompress(signature), g.publicKey, message)) {
            revert InvalidSignature();
        }

        // verify bls-aggregation signature for incentivizing worker list
        uint256[2][] memory partials = new uint256[2][](partialSignatures.length);
        uint256[4][] memory pubkeys = new uint256[4][](partialSignatures.length);
        address[] memory participantMembers = new address[](partialSignatures.length);
        for (uint256 i = 0; i < partialSignatures.length; i++) {
            if (!BLS.isValid(partialSignatures[i].partialSignature)) {
                revert InvalidPartialSignatureFormat();
            }
            partials[i] = BLS.decompress(partialSignatures[i].partialSignature);
            pubkeys[i] = g.members[partialSignatures[i].index].partialPublicKey;
            participantMembers[i] = g.members[partialSignatures[i].index].nodeIdAddress;
        }
        if (!BLS.verifyPartials(partials, pubkeys, message)) {
            revert InvalidPartialSignatures();
        }

        uint256 randomness = uint256(keccak256(abi.encode(signature)));

        // TODO implement mechanism for discovering assigned but unable group
        // if group_index != signature_task.group_index {
        //     let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

        //     late_group.fail_randomness_task_count += 1;

        //     let late_group = self.groups.get(&signature_task.group_index).unwrap();

        //     if late_group.fail_randomness_task_count >= MAX_FAIL_RANDOMNESS_TASK_COUNT
        //         && self.groups.len() > 1
        //     {
        //         let late_group_index = late_group.index;

        //         self.unresponsive_group_task = Some(UnresponsiveGroupEvent {
        //             group_index: late_group_index,
        //             assignment_block_height: self.block_height,
        //         });

        //         let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

        //         late_group.fail_randomness_task_count = 0;
        //     }
        // }

        lastOutput = randomness;

        // call user fulfill_randomness callback
        fulfillCallback(requestId, randomness, groupIndex, startGas, callback);

        // rewardRandomness for participants
        rewardRandomness(participantMembers);
    }

    function rewardRandomness(address[] memory participantMembers) internal {
        rewards[msg.sender] += s_config.committerRewardPerSignature;
        for (uint256 i = 0; i < participantMembers.length; i++) {
            rewards[participantMembers[i]] += s_config.rewardPerSignature;
        }
    }

    function fulfillCallback(
        bytes32 requestId,
        uint256 randomness,
        uint256 groupIndex,
        uint256 startGas,
        Callback memory callback
    ) internal returns (uint96 payment, bool success) {
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

        // Increment the req count for fee tier selection.
        uint64 reqCount = s_subscriptions[callback.subId].reqCount;
        s_subscriptions[callback.subId].reqCount += 1;

        // We want to charge users exactly for how much gas they use in their callback.
        // The gasAfterPaymentCalculation is meant to cover these additional operations where we
        // decrement the subscription balance and increment the groups withdrawable balance.
        // We also add the flat arpa fee to the payment amount.
        // Its specified in millionths of arpa, if s_config.fulfillmentFlatFeeArpaPPM = 1
        // 1 arpa / 1e6 = 1e18 arpa wei / 1e6 = 1e12 arpa wei.
        payment =
            calculatePaymentAmount(startGas, s_config.gasAfterPaymentCalculation, getFeeTier(reqCount), tx.gasprice);

        if (s_subscriptions[callback.subId].balance < payment) {
            revert InsufficientBalanceWhenFulfill();
        }
        s_subscriptions[callback.subId].inflightCost -= s_subscriptions[callback.subId].inflightPayments[requestId];
        delete s_subscriptions[callback.subId].inflightPayments[requestId];
        s_subscriptions[callback.subId].balance -= payment;
        // TODO mock distribute payment to working group
        s_withdrawableTokens[groupIndex] += payment;

        // Include payment in the event for tracking costs.
        emit RandomnessRequestResult(requestId, randomness, payment, success);
    }

    /**
     * @dev calls target address with exactly gasAmount gas and data as calldata
     * or reverts if at least gasAmount gas is not available.
     */
    function callWithExactGas(uint256 gasAmount, address target, bytes memory data) private returns (bool success) {
        // solhint-disable-next-line no-inline-assembly
        assembly {
            let g := gas()
            // Compute g -= GAS_FOR_CALL_EXACT_CHECK and check for underflow
            // The gas actually passed to the callee is min(gasAmount, 63//64*gas available).
            // We want to ensure that we revert if gasAmount >  63//64*gas available
            // as we do not want to provide them with less, however that check itself costs
            // gas.  GAS_FOR_CALL_EXACT_CHECK ensures we have at least enough gas to be able
            // to revert if gasAmount >  63//64*gas available.
            if lt(g, GAS_FOR_CALL_EXACT_CHECK) { revert(0, 0) }
            g := sub(g, GAS_FOR_CALL_EXACT_CHECK)
            // if g - g//64 <= gasAmount, revert
            // (we subtract g//64 because of EIP-150)
            if iszero(gt(sub(g, div(g, 64)), gasAmount)) { revert(0, 0) }
            // solidity calls check that a contract actually exists at the destination, so we do the same
            if iszero(extcodesize(target)) { revert(0, 0) }
            // call and return whether we succeeded. ignore return data
            // call(gas,addr,value,argsOffset,argsLength,retOffset,retLength)
            success := call(gasAmount, target, 0, add(data, 0x20), mload(data), 0, 0)
        }
        return success;
    }

    /**
     * @notice Sets the configuration of the adapter
     * @param minimumRequestConfirmations global min for request confirmations
     * @param maxGasLimit global max for request gas limit
     * @param stalenessSeconds if the eth/arpa feed is more stale then this, use the fallback price
     * @param gasAfterPaymentCalculation gas used in doing accounting after completing the gas measurement
     * @param fallbackWeiPerUnitArpa fallback eth/arpa price in the case of a stale feed
     * @param signatureTaskExclusiveWindow window in which a signature task is exclusive to the assigned group
     * @param rewardPerSignature reward per signature for every participating node
     * @param committerRewardPerSignature reward per signature for the committer
     * @param feeConfig fee tier configuration
     */
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
    ) external onlyOwner {
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
            gasAfterPaymentCalculation: gasAfterPaymentCalculation,
            gasExceptCallback: gasExceptCallback,
            signatureTaskExclusiveWindow: signatureTaskExclusiveWindow,
            rewardPerSignature: rewardPerSignature,
            committerRewardPerSignature: committerRewardPerSignature,
            reentrancyLock: false
        });
        s_feeConfig = feeConfig;
        s_fallbackWeiPerUnitArpa = fallbackWeiPerUnitArpa;
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

    /*
     * @notice Compute fee based on the request count
     * @param reqCount number of requests
     * @return feePPM fee in ARPA PPM
     */
    function getFeeTier(uint64 reqCount) public view returns (uint32) {
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

    // Estimate the amount of gas used for fulfillment
    function estimatePaymentAmount(
        uint256 callbackGasLimit,
        uint256 gasExceptCallback,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) public view returns (uint96) {
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

    // Get the amount of gas used for fulfillment
    function calculatePaymentAmount(
        uint256 startGas,
        uint256 gasAfterPaymentCalculation,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) public view returns (uint96) {
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
        (, weiPerUnitArpa,, timestamp,) = ARPA_ETH_FEED.latestRoundData();
        // solhint-disable-next-line not-rely-on-time
        if (staleFallback && stalenessSeconds < block.timestamp - timestamp) {
            weiPerUnitArpa = s_fallbackWeiPerUnitArpa;
        }
        return weiPerUnitArpa;
    }

    function getLastSubscription(address consumer) public view override returns (uint64) {
        return s_consumers[consumer].lastSubscription;
    }

    function getSubscription(uint64 subId)
        public
        view
        override
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

    function getPendingRequest(bytes32 requestId) public view returns (Callback memory) {
        return s_callbacks[requestId];
    }

    function getGroup(uint256 groupIndex) public view returns (Group memory) {
        return groups[groupIndex];
    }
}
