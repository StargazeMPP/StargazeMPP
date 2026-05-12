// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

interface IStargazeRegistry {
    function setReputationScore(bytes32 providerId, uint256 score) external;
}

/// @dev Minimal mirror of Chainlink's `Client.Any2EVMMessage`. Inlined here
///      so the contract compiles without pulling in the full
///      `@chainlink/contracts-ccip` dependency. At deploy time, the real
///      `CCIPReceiver` base class can be substituted via a thin shim.
struct EVMTokenAmount {
    address token;
    uint256 amount;
}

struct Any2EVMMessage {
    bytes32 messageId;
    uint64 sourceChainSelector;
    bytes sender;
    bytes data;
    EVMTokenAmount[] destTokenAmounts;
}

/// @title StargazeCcipReceiver
/// @notice Bridges reputation snapshots from `StargazeAnchor` on Solana
///         (and any future cross-chain mirror) into `StargazeRegistry` on
///         Tempo EVM via Chainlink CCIP.
///
/// Trust model:
///   - `CCIP_ROUTER_ROLE` is granted only to the official Chainlink CCIP
///     router contract for this network.
///   - Source chains and senders are explicitly allowlisted by the admin.
///   - Payload schema is `abi.encode(bytes32 providerId, uint16 score)`.
contract StargazeCcipReceiver is AccessControl {
    bytes32 public constant CCIP_ROUTER_ROLE = keccak256("CCIP_ROUTER_ROLE");

    IStargazeRegistry public immutable registry;

    /// @notice Allowlist of CCIP source chain selectors.
    mapping(uint64 selector => bool allowed) public allowedSources;

    /// @notice Per-source allowlist of sender bytes (origin program / contract address).
    mapping(uint64 selector => mapping(bytes32 senderHash => bool allowed)) public allowedSenders;

    event SourceAllowed(uint64 indexed selector, bool allowed);
    event SenderAllowed(uint64 indexed selector, bytes32 indexed senderHash, bool allowed);
    event ReputationMirrored(bytes32 indexed providerId, uint16 score, bytes32 messageId);

    error SourceNotAllowed(uint64 selector);
    error SenderNotAllowed(uint64 selector);
    error InvalidScore(uint16 score);

    constructor(address registryAddress, address admin) {
        registry = IStargazeRegistry(registryAddress);
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    function setAllowedSource(uint64 selector, bool allowed) external onlyRole(DEFAULT_ADMIN_ROLE) {
        allowedSources[selector] = allowed;
        emit SourceAllowed(selector, allowed);
    }

    function setAllowedSender(uint64 selector, bytes calldata sender, bool allowed) external onlyRole(DEFAULT_ADMIN_ROLE) {
        bytes32 h = keccak256(sender);
        allowedSenders[selector][h] = allowed;
        emit SenderAllowed(selector, h, allowed);
    }

    /// @notice Entry point for the Chainlink CCIP router. The payload must
    ///         decode to `(bytes32 providerId, uint16 score)`.
    function ccipReceive(Any2EVMMessage calldata message) external onlyRole(CCIP_ROUTER_ROLE) {
        if (!allowedSources[message.sourceChainSelector]) revert SourceNotAllowed(message.sourceChainSelector);

        bytes32 senderHash = keccak256(message.sender);
        if (!allowedSenders[message.sourceChainSelector][senderHash]) revert SenderNotAllowed(message.sourceChainSelector);

        (bytes32 providerId, uint16 score) = abi.decode(message.data, (bytes32, uint16));
        if (score > 1000) revert InvalidScore(score);

        registry.setReputationScore(providerId, uint256(score));
        emit ReputationMirrored(providerId, score, message.messageId);
    }
}
