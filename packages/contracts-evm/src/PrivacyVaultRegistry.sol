// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

interface IStargazeRegistry {
    function providers(bytes32 providerId)
        external
        view
        returns (
            address owner,
            uint256 reputationScore,
            bool registered,
            bytes32 categoryHash,
            bytes32 metaCid
        );
}

/// @title PrivacyVaultRegistry
/// @notice Tracks per-provider Groth16 verifier contract addresses,
///         buyer-key rotation config, and (optional) auditor key for
///         confidential payment compliance.
contract PrivacyVaultRegistry is AccessControl {
    /// privacyTier as bytes32 keccak256("open" | "zk-aggregate" | "confidential" | "buyer-key")
    bytes32 public constant TIER_OPEN = keccak256("open");
    bytes32 public constant TIER_ZK_AGGREGATE = keccak256("zk-aggregate");
    bytes32 public constant TIER_CONFIDENTIAL = keccak256("confidential");
    bytes32 public constant TIER_BUYER_KEY = keccak256("buyer-key");

    IStargazeRegistry public immutable stargazeRegistry;

    struct VaultConfig {
        bytes32 tier;
        address onChainVerifier;
        bytes32 arweaveCid;          // permanent storage of verifying key + circuit
        bytes32 buyerKeyRotationCid; // policy doc for per-buyer key rotation
        address auditorKey;          // optional — confidential payments compliance
        bool active;
    }

    mapping(bytes32 providerId => VaultConfig config) public configOf;

    event VaultConfigured(
        bytes32 indexed providerId,
        bytes32 indexed tier,
        address onChainVerifier,
        bytes32 arweaveCid
    );
    event AuditorKeySet(bytes32 indexed providerId, address indexed previous, address indexed current);
    event BuyerKeyRotationUpdated(bytes32 indexed providerId, bytes32 cid);
    event VaultDeactivated(bytes32 indexed providerId);

    error UnknownTier();
    error NotConfigured();
    error NotRegistered();
    error NotProviderOwner();

    constructor(address stargazeRegistryAddress, address admin) {
        stargazeRegistry = IStargazeRegistry(stargazeRegistryAddress);
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    modifier onlyProviderOwner(bytes32 providerId) {
        (address owner,, bool registered,,) = stargazeRegistry.providers(providerId);
        if (!registered) revert NotRegistered();
        if (msg.sender != owner) revert NotProviderOwner();
        _;
    }

    function configure(
        bytes32 providerId,
        bytes32 tier,
        address onChainVerifier,
        bytes32 arweaveCid
    ) external onlyProviderOwner(providerId) {
        if (
            tier != TIER_OPEN
                && tier != TIER_ZK_AGGREGATE
                && tier != TIER_CONFIDENTIAL
                && tier != TIER_BUYER_KEY
        ) revert UnknownTier();
        configOf[providerId] = VaultConfig({
            tier: tier,
            onChainVerifier: onChainVerifier,
            arweaveCid: arweaveCid,
            buyerKeyRotationCid: configOf[providerId].buyerKeyRotationCid,
            auditorKey: configOf[providerId].auditorKey,
            active: true
        });
        emit VaultConfigured(providerId, tier, onChainVerifier, arweaveCid);
    }

    function setAuditorKey(bytes32 providerId, address auditor) external onlyProviderOwner(providerId) {
        VaultConfig storage c = configOf[providerId];
        if (!c.active) revert NotConfigured();
        emit AuditorKeySet(providerId, c.auditorKey, auditor);
        c.auditorKey = auditor;
    }

    function setBuyerKeyRotationCid(bytes32 providerId, bytes32 cid) external onlyProviderOwner(providerId) {
        VaultConfig storage c = configOf[providerId];
        if (!c.active) revert NotConfigured();
        c.buyerKeyRotationCid = cid;
        emit BuyerKeyRotationUpdated(providerId, cid);
    }

    function deactivate(bytes32 providerId) external onlyRole(DEFAULT_ADMIN_ROLE) {
        VaultConfig storage c = configOf[providerId];
        if (!c.active) revert NotConfigured();
        c.active = false;
        emit VaultDeactivated(providerId);
    }
}
