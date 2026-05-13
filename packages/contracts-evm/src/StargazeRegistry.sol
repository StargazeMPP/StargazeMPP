// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {IStakeChecker} from "./IStakeChecker.sol";

/// @title StargazeRegistry
/// @notice Provider registration, reputation score storage, Verified Provider
///         badge issuance. Stake-related logic now lives on Solana; the
///         Verified Provider gate consults an external `IStakeChecker` to
///         decide whether a provider's stake (mirrored from Solana via CCIP)
///         clears the threshold.
contract StargazeRegistry is AccessControl {
    bytes32 public constant ORACLE_ROLE = keccak256("ORACLE_ROLE");
    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");

    /// @notice Reputation score threshold for Verified Provider tier.
    uint256 public constant VERIFIED_SCORE = 800;
    /// @notice Max reputation score.
    uint256 public constant MAX_REPUTATION = 1000;

    struct Provider {
        address owner;
        uint256 reputationScore;
        bool registered;
        bytes32 categoryHash; // keccak256(category string) — keep on-chain compact
        bytes32 metaCid;      // ipfs / arweave CID of the full provider profile JSON
    }

    mapping(bytes32 providerId => Provider provider) public providers;

    /// @notice External stake oracle. Returns whether a provider's mirrored
    ///         Solana stake clears the Verified Provider threshold.
    IStakeChecker public stakeChecker;

    event ProviderRegistered(bytes32 indexed providerId, address indexed owner, bytes32 categoryHash);
    event ProviderUpdated(bytes32 indexed providerId, bytes32 metaCid);
    event ReputationUpdated(bytes32 indexed providerId, uint256 score);
    event ReputationVoted(bytes32 indexed providerId, address indexed voter, bool accurate);
    event StakeCheckerSet(address indexed previous, address indexed current);

    error AlreadyRegistered();
    error NotRegistered();
    error ScoreOutOfRange();
    error NotProviderOwner();

    constructor(address admin) {
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    function register(
        bytes32 providerId,
        bytes32 categoryHash,
        bytes32 metaCid
    ) external {
        if (providers[providerId].registered) revert AlreadyRegistered();
        providers[providerId] = Provider({
            owner: msg.sender,
            reputationScore: 500, // neutral midpoint
            registered: true,
            categoryHash: categoryHash,
            metaCid: metaCid
        });
        emit ProviderRegistered(providerId, msg.sender, categoryHash);
    }

    function updateMeta(bytes32 providerId, bytes32 metaCid) external {
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        if (p.owner != msg.sender) revert NotProviderOwner();
        p.metaCid = metaCid;
        emit ProviderUpdated(providerId, metaCid);
    }

    /// @notice Oracle posts an updated composite reputation score (0–1000).
    function setReputationScore(bytes32 providerId, uint256 score) external onlyRole(ORACLE_ROLE) {
        if (score > MAX_REPUTATION) revert ScoreOutOfRange();
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        p.reputationScore = score;
        emit ReputationUpdated(providerId, score);
    }

    /// @notice Agent crowd-vote. The score math itself is computed off-chain
    ///         by the Reputation Oracle. Vote burn now happens on Solana via
    ///         CCIP fan-out when M4 lands; this function only records intent.
    function castReputationVote(bytes32 providerId, bool accurate) external {
        if (!providers[providerId].registered) revert NotRegistered();
        emit ReputationVoted(providerId, msg.sender, accurate);
    }

    /// @notice Admin-only: swap in a new `IStakeChecker` implementation.
    function setStakeChecker(address checker) external onlyRole(DEFAULT_ADMIN_ROLE) {
        emit StakeCheckerSet(address(stakeChecker), checker);
        stakeChecker = IStakeChecker(checker);
    }

    function isVerified(bytes32 providerId) external view returns (bool) {
        Provider memory p = providers[providerId];
        return p.registered
            && p.reputationScore >= VERIFIED_SCORE
            && address(stakeChecker) != address(0)
            && stakeChecker.isVerifiedStake(providerId);
    }
}
