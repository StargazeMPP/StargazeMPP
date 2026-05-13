// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Script, console2} from "forge-std/Script.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {PrivacyVaultRegistry} from "../src/PrivacyVaultRegistry.sol";
import {StargazeCcipReceiver} from "../src/StargazeCcipReceiver.sol";
import {StubStakeChecker} from "../src/StubStakeChecker.sol";
import {AggregateSumVerifier} from "../src/verifiers/AggregateSumVerifier.sol";
import {AggregateMeanVerifier} from "../src/verifiers/AggregateMeanVerifier.sol";
import {GeofenceVerifier} from "../src/verifiers/GeofenceVerifier.sol";

/// @notice Strict deployment order:
///   1. StargazeEscrow
///   2. StargazeRegistry (deployer is bootstrap admin so the script can wire
///                       the stub stake checker; admin role is then handed
///                       to the multisig and the deployer renounces).
///   3. StubStakeChecker (wired into StargazeRegistry via setStakeChecker)
///   4. PrivacyVaultRegistry (depends StargazeRegistry)
///   5. StargazeCcipReceiver (depends StargazeRegistry)
///   6. AggregateSumVerifier (shared Groth16 verifier, no deps, no auth)
///   7. AggregateMeanVerifier (shared Groth16 verifier, no deps, no auth)
///   8. GeofenceVerifier (shared Groth16 verifier, no deps, no auth)
/// Day-one admin is the 4-of-7 Safe multisig — pass its address via env.
/// Post-deploy, the admin must grant the CCIP receiver `ORACLE_ROLE` on
/// `StargazeRegistry` (cross-chain mirror). The three Groth16 verifiers are
/// pure-function contracts with no access control; no post-deploy role
/// grants are required.
contract Deploy is Script {
    function run() external {
        uint256 deployerKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address admin = vm.envAddress("ADMIN_MULTISIG");
        address pathUsd = vm.envAddress("PATHUSD_ADDRESS");
        address deployer = vm.addr(deployerKey);

        vm.startBroadcast(deployerKey);

        StargazeEscrow escrow = new StargazeEscrow(pathUsd, admin);

        // Bootstrap the registry with the deployer as admin so the script
        // itself can call `setStakeChecker`. Hand the role to the multisig
        // and renounce the bootstrap role before returning.
        StargazeRegistry registry = new StargazeRegistry(deployer);
        StubStakeChecker stakeChecker = new StubStakeChecker();
        registry.setStakeChecker(address(stakeChecker));
        registry.grantRole(registry.DEFAULT_ADMIN_ROLE(), admin);
        registry.renounceRole(registry.DEFAULT_ADMIN_ROLE(), deployer);

        PrivacyVaultRegistry vaultRegistry = new PrivacyVaultRegistry(address(registry), admin);
        StargazeCcipReceiver ccipReceiver = new StargazeCcipReceiver(address(registry), admin);
        AggregateSumVerifier aggregateSumVerifier = new AggregateSumVerifier();
        AggregateMeanVerifier aggregateMeanVerifier = new AggregateMeanVerifier();
        GeofenceVerifier geofenceVerifier = new GeofenceVerifier();

        vm.stopBroadcast();

        console2.log("StargazeEscrow        ", address(escrow));
        console2.log("StargazeRegistry      ", address(registry));
        console2.log("StubStakeChecker      ", address(stakeChecker));
        console2.log("PrivacyVaultRegistry  ", address(vaultRegistry));
        console2.log("StargazeCcipReceiver  ", address(ccipReceiver));
        console2.log("AggregateSumVerifier  ", address(aggregateSumVerifier));
        console2.log("AggregateMeanVerifier ", address(aggregateMeanVerifier));
        console2.log("GeofenceVerifier      ", address(geofenceVerifier));
    }
}
