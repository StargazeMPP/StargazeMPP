// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Script, console2} from "forge-std/Script.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {PrivacyVaultRegistry} from "../src/PrivacyVaultRegistry.sol";
import {StargazeCcipReceiver} from "../src/StargazeCcipReceiver.sol";
import {AggregateSumVerifier} from "../src/verifiers/AggregateSumVerifier.sol";
import {AggregateMeanVerifier} from "../src/verifiers/AggregateMeanVerifier.sol";
import {GeofenceVerifier} from "../src/verifiers/GeofenceVerifier.sol";

/// @notice Strict deployment order:
///   1. GAZEToken
///   2. BurnController (depends GAZEToken)
///   3. StargazeEscrow
///   4. StargazeRegistry (depends GAZEToken + BurnController)
///   5. PrivacyVaultRegistry (depends StargazeRegistry)
///   6. StargazeCcipReceiver (depends StargazeRegistry)
///   7. AggregateSumVerifier (shared Groth16 verifier, no deps, no auth)
///   8. AggregateMeanVerifier (shared Groth16 verifier, no deps, no auth)
///   9. GeofenceVerifier (shared Groth16 verifier, no deps, no auth)
/// Day-one admin is the 4-of-7 Safe multisig — pass its address via env.
/// Post-deploy, the admin must grant:
///   - the CCIP receiver ORACLE_ROLE on StargazeRegistry (cross-chain mirror)
///   - the StargazeRegistry REGISTRY_ROLE on BurnController (reputation vote burn)
/// The three Groth16 verifiers are pure-function contracts with no access control;
/// no post-deploy role grants are required.
contract Deploy is Script {
    function run() external {
        uint256 deployerKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address admin = vm.envAddress("ADMIN_MULTISIG");
        address pathUsd = vm.envAddress("PATHUSD_ADDRESS");
        uint256 initialSupply = vm.envOr("GAZE_INITIAL_SUPPLY", uint256(1_000_000_000e18));

        vm.startBroadcast(deployerKey);

        GAZEToken gaze = new GAZEToken(initialSupply, admin);
        BurnController burnController = new BurnController(address(gaze), admin);
        StargazeEscrow escrow = new StargazeEscrow(pathUsd, admin);
        StargazeRegistry registry = new StargazeRegistry(address(gaze), address(burnController), admin);
        PrivacyVaultRegistry vaultRegistry = new PrivacyVaultRegistry(address(registry), admin);
        StargazeCcipReceiver ccipReceiver = new StargazeCcipReceiver(address(registry), admin);
        AggregateSumVerifier aggregateSumVerifier = new AggregateSumVerifier();
        AggregateMeanVerifier aggregateMeanVerifier = new AggregateMeanVerifier();
        GeofenceVerifier geofenceVerifier = new GeofenceVerifier();

        vm.stopBroadcast();

        console2.log("GAZEToken             ", address(gaze));
        console2.log("BurnController        ", address(burnController));
        console2.log("StargazeEscrow        ", address(escrow));
        console2.log("StargazeRegistry      ", address(registry));
        console2.log("PrivacyVaultRegistry  ", address(vaultRegistry));
        console2.log("StargazeCcipReceiver  ", address(ccipReceiver));
        console2.log("AggregateSumVerifier  ", address(aggregateSumVerifier));
        console2.log("AggregateMeanVerifier ", address(aggregateMeanVerifier));
        console2.log("GeofenceVerifier      ", address(geofenceVerifier));
    }
}
