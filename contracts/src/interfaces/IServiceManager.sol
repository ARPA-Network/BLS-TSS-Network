// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import {ISignatureUtils} from "./ISignatureUtils.sol";
import {IRewardsCoordinator} from "./IRewardsCoordinator.sol";

interface IServiceManager {
    function registerOperator(address operator, ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature)
        external;

    function deregisterOperator(address operator) external;

    function slashDelegationStaking(address operator, uint256 amount) external;

    function getOperatorShare(address operator) external view returns (uint256);

    /**
     * @notice Creates a new rewards submission to the EigenLayer RewardsCoordinator contract, to be split amongst the
     * set of stakers delegated to operators who are registered to this `avs`
     * @param rewardsSubmissions The rewards submissions being created
     * @dev Only callabe by the permissioned rewardsInitiator address
     * @dev The duration of the `rewardsSubmission` cannot exceed `MAX_REWARDS_DURATION`
     * @dev The tokens are sent to the `RewardsCoordinator` contract
     * @dev Strategies must be in ascending order of addresses to check for duplicates
     * @dev This function will revert if the `rewardsSubmission` is malformed,
     * e.g. if the `strategies` and `weights` arrays are of non-equal lengths
     */
    function createAVSRewardsSubmission(IRewardsCoordinator.RewardsSubmission[] calldata rewardsSubmissions) external;
}
