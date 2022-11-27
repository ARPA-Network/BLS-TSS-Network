// Using the ABIEncoderV2 poses little risk here because we only use it for fetching the byte arrays
// of shares/responses/justifications
// pragma experimental ABIEncoderV2;  // don't think we need this
pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/access/Ownable.sol";

contract Coordinator is Ownable {
    /// Mapping of Ethereum Address => DKG public keys
    mapping(address => bytes) public keys;

    /// Mapping of Ethereum Address => DKG Phase 1 Shares
    mapping(address => bytes) public shares;

    /// Mapping of Ethereum Address => DKG Phase 2 Responses
    mapping(address => bytes) public responses;

    /// Mapping of Ethereum Address => DKG Phase 3 Justifications
    mapping(address => bytes) public justifications;

    // List of registered Ethereum keys (used for conveniently fetching data)
    address[] public participants;

    /// The duration of each phase
    uint256 public immutable PHASE_DURATION;

    /// The threshold of the DKG
    uint256 public immutable THRESHOLD;

    /// If it's 0 then the DKG is still pending start. If >0, it is the DKG's start block
    uint256 public startBlock = 0;

    /// A group member is one whose pubkey's length > 0
    modifier onlyGroupMember() {
        require(keys[msg.sender].length > 0, "you are not a group member!");
        _;
    }

    /// The DKG starts when startBlock > 0
    modifier onlyWhenNotStarted() {
        require(startBlock == 0, "DKG has already started");
        _;
    }

    constructor(uint256 threshold, uint256 duration) {
        THRESHOLD = threshold;
        PHASE_DURATION = duration;
    }

    function initialize(address[] calldata nodes, bytes[] calldata publicKeys)
        external
        onlyWhenNotStarted
        onlyOwner
    {
        for (uint256 i = 0; i < nodes.length; i++) {
            participants.push(nodes[i]);
            keys[nodes[i]] = publicKeys[i];
        }

        startBlock = block.number;
    }

    /// Participant publishes their data and depending on the phase the data gets inserted
    /// in the shares, responses or justifications mapping. Reverts if the participant
    /// has already published their data for a phase or if the DKG has ended.
    function publish(bytes calldata value) external onlyGroupMember {
        uint256 blocksSinceStart = block.number - startBlock;

        if (blocksSinceStart <= PHASE_DURATION) {
            require(
                shares[msg.sender].length == 0,
                "you have already published your shares"
            );
            shares[msg.sender] = value;
        } else if (blocksSinceStart <= 2 * PHASE_DURATION) {
            require(
                responses[msg.sender].length == 0,
                "you have already published your responses"
            );
            responses[msg.sender] = value;
        } else if (blocksSinceStart <= 3 * PHASE_DURATION) {
            require(
                justifications[msg.sender].length == 0,
                "you have already published your justifications"
            );
            justifications[msg.sender] = value;
        } else {
            revert("DKG Publish has ended");
        }
    }

    // Helpers to fetch data in the mappings. If a participant has registered but not
    // published their data for a phase, the array element at their index is expected to be 0

    /// Gets the participants' shares
    function getShares() external view returns (bytes[] memory) {
        bytes[] memory _shares = new bytes[](participants.length);
        for (uint256 i = 0; i < participants.length; i++) {
            _shares[i] = shares[participants[i]];
        }

        return _shares;
    }

    /// Gets the participants' responses
    function getResponses() external view returns (bytes[] memory) {
        bytes[] memory _responses = new bytes[](participants.length);
        for (uint256 i = 0; i < participants.length; i++) {
            _responses[i] = responses[participants[i]];
        }

        return _responses;
    }

    /// Gets the participants' justifications
    function getJustifications() external view returns (bytes[] memory) {
        bytes[] memory _justifications = new bytes[](participants.length);
        for (uint256 i = 0; i < participants.length; i++) {
            _justifications[i] = justifications[participants[i]];
        }

        return _justifications;
    }

    /// Gets the participants' ethereum addresses
    function getParticipants() external view returns (address[] memory) {
        return participants;
    }

    /// Gets the participants' DKG keys along with the thershold of the DKG
    function getDkgKeys() external view returns (uint256, bytes[] memory) {
        bytes[] memory _keys = new bytes[](participants.length);
        for (uint256 i = 0; i < participants.length; i++) {
            _keys[i] = keys[participants[i]];
        }

        return (THRESHOLD, _keys);
    }

    /// Returns the current phase of the DKG.
    function inPhase() public view returns (int8) {
        if (startBlock == 0) {
            return 0;
        }

        uint256 blocksSinceStart = block.number - startBlock;

        // ! Phase 0 for after deployment before initialization.

        if (blocksSinceStart <= PHASE_DURATION) {
            return 1; // share
        }

        if (blocksSinceStart <= 2 * PHASE_DURATION) {
            return 2; // response
        }

        if (blocksSinceStart <= 3 * PHASE_DURATION) {
            return 3; // justification
        }
        if (blocksSinceStart <= 4 * PHASE_DURATION) {
            return 4; // Commit DKG: Handled in controller
        }

        // ! commit_dkg Phase 4

        // revert("DKG Ended");
        return -1;
    }

    function selfDestruct() external onlyOwner {
        selfdestruct(payable(owner()));
    }
}
