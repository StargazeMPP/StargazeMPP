// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Script, console2} from "forge-std/Script.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {PrivacyVaultRegistry} from "../src/PrivacyVaultRegistry.sol";

/// @notice Strict deployment order per backend PDF §2:
///   1. GAZEToken
///   2. BurnController (depends GAZEToken)
///   3. StargazeEscrow
///   4. StargazeRegistry (depends GAZEToken + BurnController)
///   5. PrivacyVaultRegistry
/// Day-one admin is the 4-of-7 Safe multisig — pass its address via env.
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
        PrivacyVaultRegistry vaultRegistry = new PrivacyVaultRegistry(admin);

        vm.stopBroadcast();

        console2.log("GAZEToken           ", address(gaze));
        console2.log("BurnController      ", address(burnController));
        console2.log("StargazeEscrow      ", address(escrow));
        console2.log("StargazeRegistry    ", address(registry));
        console2.log("PrivacyVaultRegistry", address(vaultRegistry));
    }
}
