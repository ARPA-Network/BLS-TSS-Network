// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "./IStrategy.sol";

/**
 * @title Interface for the `IRewardsCoordinator` contract.
 * @author Layr Labs, Inc.
 * @notice Terms of Service: https://docs.eigenlayer.xyz/overview/terms-of-service
 * @notice Allows AVSs to make "Rewards Submissions", which get distributed amongst the AVSs' confirmed
 * Operators and the Stakers delegated to those Operators.
 * Calculations are performed based on the completed RewardsSubmission, with the results posted in
 * a Merkle root against which Stakers & Operators can make claims.
 */
interface IRewardsCoordinator {
    /// STRUCTS ///
    /**
     * @notice A linear combination of strategies and multipliers for AVSs to weigh
     * EigenLayer strategies.
     * @param strategy The EigenLayer strategy to be used for the rewards submission
     * @param multiplier The weight of the strategy in the rewards submission
     */
    struct StrategyAndMultiplier {
        IStrategy strategy;
        uint96 multiplier;
    }

    /**
     * Sliding Window for valid RewardsSubmission startTimestamp
     *
     * Scenario A: GENESIS_REWARDS_TIMESTAMP IS WITHIN RANGE
     *         <-----MAX_RETROACTIVE_LENGTH-----> t (block.timestamp) <---MAX_FUTURE_LENGTH--->
     *             <--------------------valid range for startTimestamp------------------------>
     *             ^
     *         GENESIS_REWARDS_TIMESTAMP
     *
     *
     * Scenario B: GENESIS_REWARDS_TIMESTAMP IS OUT OF RANGE
     *         <-----MAX_RETROACTIVE_LENGTH-----> t (block.timestamp) <---MAX_FUTURE_LENGTH--->
     *         <------------------------valid range for startTimestamp------------------------>
     *     ^
     * GENESIS_REWARDS_TIMESTAMP
     * @notice RewardsSubmission struct submitted by AVSs when making rewards for their operators and stakers
     * RewardsSubmission can be for a time range within the valid window for startTimestamp and must be within max duration.
     * See `createAVSRewardsSubmission()` for more details.
     * @param strategiesAndMultipliers The strategies and their relative weights
     * cannot have duplicate strategies and need to be sorted in ascending address order
     * @param token The rewards token to be distributed
     * @param amount The total amount of tokens to be distributed
     * @param startTimestamp The timestamp (seconds) at which the submission range is considered for distribution
     * could start in the past or in the future but within a valid range. See the diagram above.
     * @param duration The duration of the submission range in seconds. Must be <= MAX_REWARDS_DURATION
     */
    struct RewardsSubmission {
        StrategyAndMultiplier[] strategiesAndMultipliers;
        IERC20 token;
        uint256 amount;
        uint32 startTimestamp;
        uint32 duration;
    }

    /**
     * @notice A distribution root is a merkle root of the distribution of earnings for a given period.
     * The RewardsCoordinator stores all historical distribution roots so that earners can claim their earnings against older roots
     * if they wish but the merkle tree contains the cumulative earnings of all earners and tokens for a given period so earners (or their claimers if set)
     * only need to claim against the latest root to claim all available earnings.
     * @param root The merkle root of the distribution
     * @param rewardsCalculationEndTimestamp The timestamp (seconds) until which rewards have been calculated
     * @param activatedAt The timestamp (seconds) at which the root can be claimed against
     */
    struct DistributionRoot {
        bytes32 root;
        uint32 rewardsCalculationEndTimestamp;
        uint32 activatedAt;
    }

    /**
     * @notice Internal leaf in the merkle tree for the earner's account leaf
     * @param earner The address of the earner
     * @param earnerTokenRoot The merkle root of the earner's token subtree
     * Each leaf in the earner's token subtree is a TokenTreeMerkleLeaf
     */
    struct EarnerTreeMerkleLeaf {
        address earner;
        bytes32 earnerTokenRoot;
    }

    /**
     * @notice The actual leaves in the distribution merkle tree specifying the token earnings
     * for the respective earner's subtree. Each leaf is a claimable amount of a token for an earner.
     * @param token The token for which the earnings are being claimed
     * @param cumulativeEarnings The cumulative earnings of the earner for the token
     */
    struct TokenTreeMerkleLeaf {
        IERC20 token;
        uint256 cumulativeEarnings;
    }

    /**
     *
     *                         VIEW FUNCTIONS
     *
     */

    /// @notice The address of the entity that can update the contract with new merkle roots
    function rewardsUpdater() external view returns (address);

    /**
     * @notice The interval in seconds at which the calculation for a RewardsSubmission distribution is done.
     * @dev Rewards Submission durations must be multiples of this interval.
     */
    function CALCULATION_INTERVAL_SECONDS() external view returns (uint32);

    /// @notice The maximum amount of time (seconds) that a RewardsSubmission can span over
    function MAX_REWARDS_DURATION() external view returns (uint32);

    /// @notice max amount of time (seconds) that a submission can start in the past
    function MAX_RETROACTIVE_LENGTH() external view returns (uint32);

    /// @notice max amount of time (seconds) that a submission can start in the future
    function MAX_FUTURE_LENGTH() external view returns (uint32);

    /// @notice absolute min timestamp (seconds) that a submission can start at
    function GENESIS_REWARDS_TIMESTAMP() external view returns (uint32);

    /// @notice Delay in timestamp (seconds) before a posted root can be claimed against
    function activationDelay() external view returns (uint32);

    /// @notice Mapping: earner => the address of the entity who can call `processClaim` on behalf of the earner
    function claimerFor(address earner) external view returns (address);

    /// @notice Mapping: claimer => token => total amount claimed
    function cumulativeClaimed(address claimer, IERC20 token) external view returns (uint256);

    /// @notice the commission for all operators across all avss
    function globalOperatorCommissionBips() external view returns (uint16);

    /// @notice the commission for a specific operator for a specific avs
    /// NOTE: Currently unused and simply returns the globalOperatorCommissionBips value but will be used in future release
    function operatorCommissionBips(address operator, address avs) external view returns (uint16);

    /// @notice return the hash of the earner's leaf
    function calculateEarnerLeafHash(EarnerTreeMerkleLeaf calldata leaf) external pure returns (bytes32);

    /// @notice returns the hash of the earner's token leaf
    function calculateTokenLeafHash(TokenTreeMerkleLeaf calldata leaf) external pure returns (bytes32);

    /// @notice The timestamp until which RewardsSubmissions have been calculated
    function currRewardsCalculationEndTimestamp() external view returns (uint32);

    /// @notice loop through distribution roots from reverse and return hash
    function getRootIndexFromHash(bytes32 rootHash) external view returns (uint32);

    /// @notice returns the number of distribution roots posted
    function getDistributionRootsLength() external view returns (uint256);

    /**
     *
     *                         EXTERNAL FUNCTIONS
     *
     */

    /**
     * @notice Creates a new rewards submission on behalf of an AVS, to be split amongst the
     * set of stakers delegated to operators who are registered to the `avs`
     * @param rewardsSubmissions The rewards submissions being created
     * @dev Expected to be called by the ServiceManager of the AVS on behalf of which the submission is being made
     * @dev The duration of the `rewardsSubmission` cannot exceed `MAX_REWARDS_DURATION`
     * @dev The tokens are sent to the `RewardsCoordinator` contract
     * @dev Strategies must be in ascending order of addresses to check for duplicates
     * @dev This function will revert if the `rewardsSubmission` is malformed,
     * e.g. if the `strategies` and `weights` arrays are of non-equal lengths
     */
    function createAVSRewardsSubmission(RewardsSubmission[] calldata rewardsSubmissions) external;

    /**
     * @notice similar to `createAVSRewardsSubmission` except the rewards are split amongst *all* stakers
     * rather than just those delegated to operators who are registered to a single avs and is
     * a permissioned call based on isRewardsForAllSubmitter mapping.
     */
    function createRewardsForAllSubmission(RewardsSubmission[] calldata rewardsSubmission) external;
}
