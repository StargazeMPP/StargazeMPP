// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {IStakeChecker} from "./IStakeChecker.sol";

/// @dev Minimal mirror of Chainlink's `Client.EVMTokenAmount`. Duplicated here
///      (rather than imported from `StargazeCcipReceiver.sol`) so this contract
///      stays decoupled from the reputation-mirror receiver. Both files inline
///      the same CCIP types and can be swapped to the real `CCIPReceiver` base
///      class via a thin shim at deploy time.
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

/// @title StargazeStakeMirror
/// @notice Receives per-staker GAZE stake snapshots from the Solana side
///         (`dispatch_stake_to_tempo`) via Chainlink CCIP and exposes the
///         aggregate per provider through `IStakeChecker.isVerifiedStake`.
///
/// Trust model:
///   - `CCIP_ROUTER_ROLE` is granted only to the official Chainlink CCIP
///     router contract for this network.
///   - Source chains and senders are explicitly allowlisted by the admin.
///   - Payload schema is
///     `abi.encode(bytes32 providerId, address owner, uint256 amount)` (96
///     bytes). The `owner` field is the Solana staker's public key truncated
///     to its bottom 20 bytes — this mirror treats it as an opaque per-staker
///     key and never as an EVM address. Two distinct Solana stakers whose
///     truncated keys collide on Tempo would be aggregated together; the
///     space is `2**160` so the collision probability is negligible.
///   - `verifiedThreshold` is denominated in base units of the Solana SPL
///     (6 decimals), i.e. `500_000_000` = 500 GAZE.
contract StargazeStakeMirror is AccessControl, IStakeChecker {
    bytes32 public constant CCIP_ROUTER_ROLE = keccak256("CCIP_ROUTER_ROLE");

    /// @notice Allowlist of CCIP source chain selectors.
    mapping(uint64 selector => bool allowed) public allowedSources;

    /// @notice Per-source allowlist of sender bytes (origin program / contract address).
    mapping(uint64 selector => mapping(bytes32 senderHash => bool allowed)) public allowedSenders;

    /// @notice Per-staker mirrored balance, keyed by `(providerId, owner)`.
    mapping(bytes32 providerId => mapping(address owner => uint256 amount)) public stakeOf;

    /// @notice Running aggregate per provider, the sum of every staker's mirror.
    mapping(bytes32 providerId => uint256 total) public totalStake;

    /// @notice Minimum aggregate stake (in SPL base units, 6 decimals) that
    ///         clears `isVerifiedStake`. Mutable by the admin.
    uint256 public verifiedThreshold;

    event SourceAllowed(uint64 indexed selector, bool allowed);
    event SenderAllowed(uint64 indexed selector, bytes32 indexed senderHash, bool allowed);
    event StakeMirrored(
        bytes32 indexed providerId, address indexed owner, uint256 amount, uint256 totalStake, bytes32 messageId
    );
    event VerifiedThresholdSet(uint256 previous, uint256 current);

    error SourceNotAllowed(uint64 selector);
    error SenderNotAllowed(uint64 selector);

    constructor(address admin, uint256 initialThreshold) {
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        verifiedThreshold = initialThreshold;
        emit VerifiedThresholdSet(0, initialThreshold);
    }

    function setAllowedSource(uint64 selector, bool allowed) external onlyRole(DEFAULT_ADMIN_ROLE) {
        allowedSources[selector] = allowed;
        emit SourceAllowed(selector, allowed);
    }

    function setAllowedSender(uint64 selector, bytes calldata sender, bool allowed)
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        bytes32 h = keccak256(sender);
        allowedSenders[selector][h] = allowed;
        emit SenderAllowed(selector, h, allowed);
    }

    function setVerifiedThreshold(uint256 threshold) external onlyRole(DEFAULT_ADMIN_ROLE) {
        uint256 previous = verifiedThreshold;
        verifiedThreshold = threshold;
        emit VerifiedThresholdSet(previous, threshold);
    }

    /// @notice Entry point for the Chainlink CCIP router. The payload must
    ///         decode to `(bytes32 providerId, address owner, uint256 amount)`.
    function ccipReceive(Any2EVMMessage calldata message) external onlyRole(CCIP_ROUTER_ROLE) {
        if (!allowedSources[message.sourceChainSelector]) revert SourceNotAllowed(message.sourceChainSelector);

        bytes32 senderHash = keccak256(message.sender);
        if (!allowedSenders[message.sourceChainSelector][senderHash]) {
            revert SenderNotAllowed(message.sourceChainSelector);
        }

        (bytes32 providerId, address owner, uint256 newAmount) = abi.decode(message.data, (bytes32, address, uint256));

        uint256 oldAmount = stakeOf[providerId][owner];
        // Solidity 0.8 arithmetic reverts on overflow/underflow; no `unchecked`
        // blocks are required for these checked deltas.
        if (newAmount > oldAmount) {
            totalStake[providerId] += (newAmount - oldAmount);
        } else if (newAmount < oldAmount) {
            totalStake[providerId] -= (oldAmount - newAmount);
        }
        stakeOf[providerId][owner] = newAmount;

        emit StakeMirrored(providerId, owner, newAmount, totalStake[providerId], message.messageId);
    }

    /// @inheritdoc IStakeChecker
    function isVerifiedStake(bytes32 providerId) external view returns (bool) {
        return totalStake[providerId] >= verifiedThreshold;
    }
}
