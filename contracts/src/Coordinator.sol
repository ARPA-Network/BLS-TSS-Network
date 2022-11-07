// Using the ABIEncoderV2 poses little risk here because we only use it for fetching the byte arrays
// of shares/responses/justifications
// pragma experimental ABIEncoderV2;  // don't think we need this
pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/access/Ownable.sol";

contract Coordinator is Ownable {
    // enum UserState {
    //     CannotRegister,
    //     CanRegister,
    //     Registered
    // }

    /// Mapping of Ethereum Address => UserState for the actions a user can do
    // mapping(address => UserState) public userState;

    /// Mapping of Ethereum Address => BLS public keys
    mapping(address => bytes) public keys;

    /// Mapping of Ethereum Address => DKG Phase 1 Shares
    mapping(address => bytes) public shares;

    /// Mapping of Ethereum Address => DKG Phase 2 Responses
    mapping(address => bytes) public responses;

    /// Mapping of Ethereum Address => DKG Phase 3 Justifications
    mapping(address => bytes) public justifications;

    // List of registered Ethereum keys (used for conveniently fetching data)
    address[] public participants;

    //! Added participants map for more efficient "onlyRegistered"
    mapping(address => bool) public participant_map;

    /// The duration of each phase
    uint256 public immutable PHASE_DURATION;

    /// The threshold of the DKG
    uint256 public immutable THRESHOLD;

    /// If it's 0 then the DKG is still pending start. If >0, it is the DKG's start block
    uint256 public startBlock = 0;

    /// The owner of the DKG is the address which can call the `start` function
    // address public owner;

    /// A registered participant is one whose pubkey's length > 0
    modifier onlyRegistered() {
        // require(
        //     userState[msg.sender] == UserState.Registered,
        //     "you are not registered!"
        // );
        require(participant_map[msg.sender], "you are not registered!");
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
        // owner = msg.sender;
    }

    // ! Start done via initialize() in randcast
    // /// Kickoff function which starts the counter (INITIALIZE)
    // function start() external onlyWhenNotStarted onlyOwner {
    //     // require(msg.sender == owner, "only owner may start the DKG");
    //     startBlock = block.number;
    // }

    // ! Allowlist (DKG Nodes) done via initialize() in randcast
    // /// The administrator must allowlist an addrss for participation in the DKG
    // function allowlist(address user) external onlyWhenNotStarted onlyOwner {
    //     // require(msg.sender == owner, "only owner may allowlist users");

    //     require(
    //         userState[user] == UserState.CannotRegister,
    //         "user is already allowlisted"
    //     );
    //     userState[user] = UserState.CanRegister;
    // }

    //! Register done via initialize() in randcast
    /// This function ties a DKG participant's on-chain address with their BLS Public Key
    // function register(bytes calldata blsPublicKey) external onlyWhenNotStarted {
    //     require(
    //         userState[msg.sender] == UserState.CanRegister,
    //         "user is not allowlisted or has already registered"
    //     );

    //     participants.push(msg.sender);
    //     keys[msg.sender] = blsPublicKey;

    //     // the user is now registered
    //     userState[msg.sender] = UserState.Registered;
    // }

    //! New initialize code here (replaces start() and register() and allowlist())
    // struct Member {
    //     address node_address;
    //     // uint256 node_index; // index of node within group
    //     bytes blsPublicKey;
    // }

    function initialize(address[] calldata nodes, bytes[] calldata publicKeys)
        external
        onlyWhenNotStarted
        // group.epoch # do we really need this in the contract?
        onlyOwner
    {
        // the below might need some work.
        for (uint256 i = 0; i < nodes.length; i++) {
            participant_map[nodes[i]] = true;
            participants.push(nodes[i]);
            keys[nodes[i]] = publicKeys[i];
        }

        startBlock = block.number;
    }

    /// Participant publishes their data and depending on the phase the data gets inserted
    /// in the shares, responses or justifications mapping. Reverts if the participant
    /// has already published their data for a phase or if the DKG has ended.
    function publish(bytes calldata value) external onlyRegistered {
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

    /// Gets the participants' BLS keys along with the thershold of the DKG
    function getBlsKeys() external view returns (uint256, bytes[] memory) {
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
}
