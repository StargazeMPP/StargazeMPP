// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {GAZEToken} from "../src/GAZEToken.sol";

contract GAZETokenTest is Test {
    GAZEToken internal gaze;
    address internal admin = address(0xA11CE);
    address internal alice = address(0xA1);

    function setUp() public {
        gaze = new GAZEToken(1_000_000e18, admin);
        vm.prank(admin);
        gaze.transfer(alice, 1_000e18);
    }

    function test_StakeFlow() public {
        vm.startPrank(alice);
        gaze.stake(100e18);
        assertEq(gaze.balanceOf(alice), 900e18);
        (uint256 amount,,,) = gaze.stakeOf(alice);
        assertEq(amount, 100e18);

        gaze.requestUnstake(40e18);
        vm.expectRevert();
        gaze.claimUnstake();

        skip(7 days + 1);
        gaze.claimUnstake();
        assertEq(gaze.balanceOf(alice), 940e18);
        vm.stopPrank();
    }
}
