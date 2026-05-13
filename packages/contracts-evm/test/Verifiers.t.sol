// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {AggregateSumVerifier} from "../src/verifiers/AggregateSumVerifier.sol";
import {AggregateMeanVerifier} from "../src/verifiers/AggregateMeanVerifier.sol";
import {GeofenceVerifier} from "../src/verifiers/GeofenceVerifier.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {PrivacyVaultRegistry} from "../src/PrivacyVaultRegistry.sol";

interface IGroth16Like {
    function verifyProof(
        uint256[2] calldata a,
        uint256[2][2] calldata b,
        uint256[2] calldata c,
        uint256[1] calldata pubSignals
    ) external view returns (bool);
}

contract VerifiersTest is Test {
    AggregateSumVerifier internal aggregate;
    AggregateMeanVerifier internal mean;
    GeofenceVerifier internal geofence;

    string internal aggregateVector;
    string internal meanVector;
    string internal geofenceVector;

    function setUp() public {
        aggregate = new AggregateSumVerifier();
        mean = new AggregateMeanVerifier();
        geofence = new GeofenceVerifier();
        aggregateVector = vm.readFile("./test/vectors/aggregate_sum.json");
        meanVector = vm.readFile("./test/vectors/aggregate_mean.json");
        geofenceVector = vm.readFile("./test/vectors/geofence.json");
    }

    function _loadPoint(string memory raw, string memory key) internal pure returns (uint256[2] memory out) {
        uint256[] memory v = vm.parseJsonUintArray(raw, key);
        require(v.length == 2, "expected 2-element G1 point");
        out[0] = v[0];
        out[1] = v[1];
    }

    function _loadG2(string memory raw, string memory key) internal pure returns (uint256[2][2] memory out) {
        uint256[] memory row0 = vm.parseJsonUintArray(raw, string.concat(key, "[0]"));
        uint256[] memory row1 = vm.parseJsonUintArray(raw, string.concat(key, "[1]"));
        require(row0.length == 2 && row1.length == 2, "expected 2x2 G2 element");
        out[0][0] = row0[0];
        out[0][1] = row0[1];
        out[1][0] = row1[0];
        out[1][1] = row1[1];
    }

    function test_AggregateSumVerifier_AcceptsKnownGoodProof() public view {
        uint256[2] memory a = _loadPoint(aggregateVector, ".a");
        uint256[2][2] memory b = _loadG2(aggregateVector, ".b");
        uint256[2] memory c = _loadPoint(aggregateVector, ".c");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(aggregateVector, ".pubSignals");
        require(signalsDyn.length == 1, "aggregate has one public signal");
        uint256[1] memory pubSignals = [signalsDyn[0]];

        assertTrue(aggregate.verifyProof(a, b, c, pubSignals), "valid proof must verify");
        assertEq(pubSignals[0], 36, "public signal is the claimed sum 1+...+8");
    }

    function test_AggregateSumVerifier_RejectsTamperedPublicSignal() public view {
        uint256[2] memory a = _loadPoint(aggregateVector, ".a");
        uint256[2][2] memory b = _loadG2(aggregateVector, ".b");
        uint256[2] memory c = _loadPoint(aggregateVector, ".c");
        uint256[1] memory pubSignals = [uint256(35)]; // off-by-one — must reject

        assertFalse(aggregate.verifyProof(a, b, c, pubSignals), "tampered signal must not verify");
    }

    function test_AggregateMeanVerifier_AcceptsKnownGoodProof() public view {
        uint256[2] memory a = _loadPoint(meanVector, ".a");
        uint256[2][2] memory b = _loadG2(meanVector, ".b");
        uint256[2] memory c = _loadPoint(meanVector, ".c");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(meanVector, ".pubSignals");
        require(signalsDyn.length == 1, "mean has one public signal");
        uint256[1] memory pubSignals = [signalsDyn[0]];

        assertTrue(mean.verifyProof(a, b, c, pubSignals), "valid mean proof must verify");
        assertEq(pubSignals[0], 10, "claimed mean of 3..17 (step 2) is 10");
    }

    function test_AggregateMeanVerifier_RejectsTamperedPublicSignal() public view {
        uint256[2] memory a = _loadPoint(meanVector, ".a");
        uint256[2][2] memory b = _loadG2(meanVector, ".b");
        uint256[2] memory c = _loadPoint(meanVector, ".c");
        uint256[1] memory pubSignals = [uint256(11)]; // off-by-one mean

        assertFalse(mean.verifyProof(a, b, c, pubSignals), "tampered mean must not verify");
    }

    function test_AggregateSumVerifier_RejectsTamperedProof() public view {
        uint256[2] memory a = _loadPoint(aggregateVector, ".a");
        uint256[2][2] memory b = _loadG2(aggregateVector, ".b");
        uint256[2] memory c = _loadPoint(aggregateVector, ".c");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(aggregateVector, ".pubSignals");
        uint256[1] memory pubSignals = [signalsDyn[0]];

        a[0] = addmod(a[0], 1, type(uint256).max);

        assertFalse(aggregate.verifyProof(a, b, c, pubSignals), "tampered proof must not verify");
    }

    function test_GeofenceVerifier_AcceptsKnownGoodProof() public view {
        uint256[2] memory a = _loadPoint(geofenceVector, ".a");
        uint256[2][2] memory b = _loadG2(geofenceVector, ".b");
        uint256[2] memory c = _loadPoint(geofenceVector, ".c");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(geofenceVector, ".pubSignals");
        require(signalsDyn.length == 4, "geofence has four public signals");
        uint256[4] memory pubSignals =
            [signalsDyn[0], signalsDyn[1], signalsDyn[2], signalsDyn[3]];

        assertTrue(geofence.verifyProof(a, b, c, pubSignals), "valid proof must verify");
        // Sanity check the encoded bounding box surrounds the encoded point —
        // the public signals are [minLat, maxLat, minLon, maxLon] (micro-deg + 2^31 offset).
        assertLt(pubSignals[0], pubSignals[1], "minLat < maxLat");
        assertLt(pubSignals[2], pubSignals[3], "minLon < maxLon");
    }

    function test_GeofenceVerifier_RejectsExpandedBox() public view {
        uint256[2] memory a = _loadPoint(geofenceVector, ".a");
        uint256[2][2] memory b = _loadG2(geofenceVector, ".b");
        uint256[2] memory c = _loadPoint(geofenceVector, ".c");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(geofenceVector, ".pubSignals");
        // Stretch maxLat by 1 µdeg → public signals no longer match the
        // ones the prover committed to, so verification must fail.
        uint256[4] memory pubSignals =
            [signalsDyn[0], signalsDyn[1] + 1, signalsDyn[2], signalsDyn[3]];

        assertFalse(geofence.verifyProof(a, b, c, pubSignals), "altered box must not verify");
    }
}

/// @notice Confirms the verifier can be discovered and invoked through the
/// PrivacyVaultRegistry config — the production round-trip a buyer would
/// follow when verifying a provider's published proof on-chain.
contract VerifierWiringTest is Test {
    GAZEToken internal gaze;
    BurnController internal bc;
    StargazeRegistry internal stargaze;
    PrivacyVaultRegistry internal vaultRegistry;
    AggregateSumVerifier internal aggregateVerifier;

    address internal admin = address(0xA11CE);
    address internal provider = address(0xBEEF);

    bytes32 internal constant PROVIDER_ID = keccak256("axonmed");
    bytes32 internal constant CATEGORY = keccak256("health");
    bytes32 internal constant META_CID = bytes32(uint256(0xCAFE));
    bytes32 internal constant ARWEAVE_CID = keccak256("ar://aggregate-vk");

    function setUp() public {
        gaze = new GAZEToken(1_000_000e18, admin);
        bc = new BurnController(address(gaze), admin);
        stargaze = new StargazeRegistry(address(gaze), address(bc), admin);
        vaultRegistry = new PrivacyVaultRegistry(address(stargaze), admin);
        aggregateVerifier = new AggregateSumVerifier();

        vm.startPrank(admin);
        gaze.setBurnController(address(bc));
        gaze.transfer(provider, 10_000e18);
        vm.stopPrank();

        uint256 stake = stargaze.MIN_STAKE();
        vm.prank(provider);
        gaze.approve(address(stargaze), stake);
        vm.prank(provider);
        stargaze.register(PROVIDER_ID, CATEGORY, META_CID, stake);

        // Cache view return before the prank — a view call would otherwise
        // consume the vm.prank cheat before configure() runs.
        bytes32 tier = vaultRegistry.TIER_ZK_AGGREGATE();
        vm.prank(provider);
        vaultRegistry.configure(PROVIDER_ID, tier, address(aggregateVerifier), ARWEAVE_CID);
    }

    function test_BuyerCanLookUpAndCallVerifier() public view {
        (, address onChainVerifier,,,, bool active) = vaultRegistry.configOf(PROVIDER_ID);
        assertTrue(active, "vault active");
        assertEq(onChainVerifier, address(aggregateVerifier), "registry returns the verifier");

        string memory raw = vm.readFile("./test/vectors/aggregate_sum.json");

        uint256[] memory aDyn = vm.parseJsonUintArray(raw, ".a");
        uint256[] memory cDyn = vm.parseJsonUintArray(raw, ".c");
        uint256[] memory row0 = vm.parseJsonUintArray(raw, ".b[0]");
        uint256[] memory row1 = vm.parseJsonUintArray(raw, ".b[1]");
        uint256[] memory signalsDyn = vm.parseJsonUintArray(raw, ".pubSignals");

        uint256[2] memory a = [aDyn[0], aDyn[1]];
        uint256[2] memory c = [cDyn[0], cDyn[1]];
        uint256[2][2] memory b;
        b[0][0] = row0[0];
        b[0][1] = row0[1];
        b[1][0] = row1[0];
        b[1][1] = row1[1];
        uint256[1] memory pubSignals = [signalsDyn[0]];

        // Call through the dynamically-resolved verifier address — the
        // exact code path a buyer-side client would take in production.
        assertTrue(IGroth16Like(onChainVerifier).verifyProof(a, b, c, pubSignals));
    }
}
