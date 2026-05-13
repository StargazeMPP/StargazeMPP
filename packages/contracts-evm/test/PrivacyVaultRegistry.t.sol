// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {PrivacyVaultRegistry} from "../src/PrivacyVaultRegistry.sol";

contract PrivacyVaultRegistryTest is Test {
    GAZEToken internal gaze;
    BurnController internal bc;
    StargazeRegistry internal stargaze;
    PrivacyVaultRegistry internal registry;

    address internal admin = address(0xA11CE);
    address internal provider = address(0xBEEF);
    address internal attacker = address(0xBAD);
    address internal verifierA = address(0xFE1);
    address internal verifierB = address(0xFE2);
    address internal auditorA = address(0xAD1);
    address internal auditorB = address(0xAD2);

    bytes32 internal constant PROVIDER_ID = keccak256("mpp32");
    bytes32 internal constant CATEGORY = keccak256("search");
    bytes32 internal constant META_CID = bytes32(uint256(0xCAFE));
    bytes32 internal constant ARWEAVE_CID = keccak256("ar://vk-and-circuit");
    bytes32 internal constant ROTATION_CID = keccak256("ar://rotation-policy");

    uint256 internal constant INITIAL_SUPPLY = 1_000_000e18;

    event VaultConfigured(
        bytes32 indexed providerId,
        bytes32 indexed tier,
        address onChainVerifier,
        bytes32 arweaveCid
    );
    event AuditorKeySet(bytes32 indexed providerId, address indexed previous, address indexed current);
    event BuyerKeyRotationUpdated(bytes32 indexed providerId, bytes32 cid);
    event VaultDeactivated(bytes32 indexed providerId);

    function setUp() public {
        gaze = new GAZEToken(INITIAL_SUPPLY, admin);
        bc = new BurnController(address(gaze), admin);
        stargaze = new StargazeRegistry(address(gaze), address(bc), admin);
        registry = new PrivacyVaultRegistry(address(stargaze), admin);

        vm.startPrank(admin);
        gaze.setBurnController(address(bc));
        gaze.transfer(provider, 10_000e18);
        vm.stopPrank();

        // Register the provider in StargazeRegistry so PrivacyVaultRegistry
        // can resolve ownership.
        uint256 stake = stargaze.MIN_STAKE();
        vm.prank(provider);
        gaze.approve(address(stargaze), stake);
        vm.prank(provider);
        stargaze.register(PROVIDER_ID, CATEGORY, META_CID, stake);
    }

    function _readConfig(bytes32 id)
        internal
        view
        returns (
            bytes32 tier,
            address onChainVerifier,
            bytes32 arweaveCid,
            bytes32 buyerKeyRotationCid,
            address auditorKey,
            bool active
        )
    {
        (tier, onChainVerifier, arweaveCid, buyerKeyRotationCid, auditorKey, active) = registry.configOf(id);
    }

    function test_Configure_HappyPath_OpenTier() public {
        bytes32 tier = registry.TIER_OPEN();

        vm.expectEmit(true, true, false, true, address(registry));
        emit VaultConfigured(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        (
            bytes32 storedTier,
            address storedVerifier,
            bytes32 storedCid,
            bytes32 storedRotation,
            address storedAuditor,
            bool active
        ) = _readConfig(PROVIDER_ID);

        assertEq(storedTier, tier);
        assertEq(storedVerifier, verifierA);
        assertEq(storedCid, ARWEAVE_CID);
        assertEq(storedRotation, bytes32(0));
        assertEq(storedAuditor, address(0));
        assertTrue(active);
    }

    function test_Configure_HappyPath_AllFourTiers() public {
        bytes32[4] memory tiers = [
            registry.TIER_OPEN(),
            registry.TIER_ZK_AGGREGATE(),
            registry.TIER_CONFIDENTIAL(),
            registry.TIER_BUYER_KEY()
        ];

        for (uint256 i = 0; i < tiers.length; ++i) {
            vm.prank(provider);
            registry.configure(PROVIDER_ID, tiers[i], verifierA, ARWEAVE_CID);

            (bytes32 storedTier,,,,, bool active) = _readConfig(PROVIDER_ID);
            assertEq(storedTier, tiers[i]);
            assertTrue(active);
        }
    }

    function test_Configure_RevertsOnUnknownTier() public {
        bytes32 bogus = keccak256("not-a-tier");
        vm.prank(provider);
        vm.expectRevert(PrivacyVaultRegistry.UnknownTier.selector);
        registry.configure(PROVIDER_ID, bogus, verifierA, ARWEAVE_CID);
    }

    function test_Configure_RevertsOnAttacker() public {
        bytes32 tier = registry.TIER_OPEN();

        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.prank(attacker);
        vm.expectRevert(PrivacyVaultRegistry.NotProviderOwner.selector);
        registry.configure(PROVIDER_ID, tier, verifierB, ARWEAVE_CID);

        (, address storedVerifier,,,, bool active) = _readConfig(PROVIDER_ID);
        assertEq(storedVerifier, verifierA, "config unchanged");
        assertTrue(active);
    }

    function test_Configure_RevertsWhenProviderNotRegistered() public {
        bytes32 unknownId = keccak256("ghost");
        bytes32 tier = registry.TIER_OPEN();

        vm.prank(provider);
        vm.expectRevert(PrivacyVaultRegistry.NotRegistered.selector);
        registry.configure(unknownId, tier, verifierA, ARWEAVE_CID);
    }

    function test_Configure_PreservesAuditorAndRotationCid() public {
        bytes32 openTier = registry.TIER_OPEN();
        bytes32 confidentialTier = registry.TIER_CONFIDENTIAL();

        vm.prank(provider);
        registry.configure(PROVIDER_ID, openTier, verifierA, ARWEAVE_CID);

        vm.prank(provider);
        registry.setAuditorKey(PROVIDER_ID, auditorA);

        vm.prank(provider);
        registry.setBuyerKeyRotationCid(PROVIDER_ID, ROTATION_CID);

        // Re-configure with a new tier + verifier; auditor + rotation cid must survive.
        vm.prank(provider);
        registry.configure(PROVIDER_ID, confidentialTier, verifierB, ARWEAVE_CID);

        (
            bytes32 storedTier,
            address storedVerifier,
            ,
            bytes32 storedRotation,
            address storedAuditor,
            bool active
        ) = _readConfig(PROVIDER_ID);

        assertEq(storedTier, confidentialTier);
        assertEq(storedVerifier, verifierB);
        assertEq(storedRotation, ROTATION_CID);
        assertEq(storedAuditor, auditorA);
        assertTrue(active);
    }

    function test_SetAuditorKey_HappyPath() public {
        bytes32 tier = registry.TIER_CONFIDENTIAL();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.expectEmit(true, true, true, true, address(registry));
        emit AuditorKeySet(PROVIDER_ID, address(0), auditorA);
        vm.prank(provider);
        registry.setAuditorKey(PROVIDER_ID, auditorA);

        (,,,, address storedAuditor,) = _readConfig(PROVIDER_ID);
        assertEq(storedAuditor, auditorA);
    }

    function test_SetAuditorKey_RevertsOnAttacker() public {
        bytes32 tier = registry.TIER_CONFIDENTIAL();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.prank(attacker);
        vm.expectRevert(PrivacyVaultRegistry.NotProviderOwner.selector);
        registry.setAuditorKey(PROVIDER_ID, auditorB);

        (,,,, address storedAuditor,) = _readConfig(PROVIDER_ID);
        assertEq(storedAuditor, address(0), "auditor unchanged");
    }

    function test_SetAuditorKey_RevertsWhenNotActive() public {
        // Provider is registered but vault never configured → NotConfigured.
        vm.prank(provider);
        vm.expectRevert(PrivacyVaultRegistry.NotConfigured.selector);
        registry.setAuditorKey(PROVIDER_ID, auditorA);
    }

    function test_SetAuditorKey_OverwriteEmitsPrevious() public {
        bytes32 tier = registry.TIER_CONFIDENTIAL();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.prank(provider);
        registry.setAuditorKey(PROVIDER_ID, auditorA);

        vm.expectEmit(true, true, true, true, address(registry));
        emit AuditorKeySet(PROVIDER_ID, auditorA, auditorB);
        vm.prank(provider);
        registry.setAuditorKey(PROVIDER_ID, auditorB);

        (,,,, address storedAuditor,) = _readConfig(PROVIDER_ID);
        assertEq(storedAuditor, auditorB);
    }

    function test_SetBuyerKeyRotationCid_HappyPath() public {
        bytes32 tier = registry.TIER_BUYER_KEY();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.expectEmit(true, false, false, true, address(registry));
        emit BuyerKeyRotationUpdated(PROVIDER_ID, ROTATION_CID);
        vm.prank(provider);
        registry.setBuyerKeyRotationCid(PROVIDER_ID, ROTATION_CID);

        (,,, bytes32 storedRotation,,) = _readConfig(PROVIDER_ID);
        assertEq(storedRotation, ROTATION_CID);
    }

    function test_SetBuyerKeyRotationCid_RevertsOnAttacker() public {
        bytes32 tier = registry.TIER_BUYER_KEY();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.prank(attacker);
        vm.expectRevert(PrivacyVaultRegistry.NotProviderOwner.selector);
        registry.setBuyerKeyRotationCid(PROVIDER_ID, ROTATION_CID);

        (,,, bytes32 storedRotation,,) = _readConfig(PROVIDER_ID);
        assertEq(storedRotation, bytes32(0), "rotation cid unchanged");
    }

    function test_SetBuyerKeyRotationCid_RevertsWhenNotActive() public {
        vm.prank(provider);
        vm.expectRevert(PrivacyVaultRegistry.NotConfigured.selector);
        registry.setBuyerKeyRotationCid(PROVIDER_ID, ROTATION_CID);
    }

    function test_Deactivate_HappyPath() public {
        bytes32 tier = registry.TIER_OPEN();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.expectEmit(true, false, false, true, address(registry));
        emit VaultDeactivated(PROVIDER_ID);
        vm.prank(admin);
        registry.deactivate(PROVIDER_ID);

        (,,,,, bool active) = _readConfig(PROVIDER_ID);
        assertFalse(active);
    }

    function test_Deactivate_OnlyAdmin() public {
        bytes32 tier = registry.TIER_OPEN();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        bytes32 adminRole = registry.DEFAULT_ADMIN_ROLE();
        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, attacker, adminRole)
        );
        registry.deactivate(PROVIDER_ID);
    }

    function test_Deactivate_RevertsWhenAlreadyInactive() public {
        bytes32 tier = registry.TIER_OPEN();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, ARWEAVE_CID);

        vm.prank(admin);
        registry.deactivate(PROVIDER_ID);

        vm.prank(admin);
        vm.expectRevert(PrivacyVaultRegistry.NotConfigured.selector);
        registry.deactivate(PROVIDER_ID);
    }

    function testFuzz_RoundtripsArbitraryCid(bytes32 cid) public {
        bytes32 tier = registry.TIER_OPEN();
        vm.prank(provider);
        registry.configure(PROVIDER_ID, tier, verifierA, cid);

        (,, bytes32 storedCid,,,) = _readConfig(PROVIDER_ID);
        assertEq(storedCid, cid);
    }
}
