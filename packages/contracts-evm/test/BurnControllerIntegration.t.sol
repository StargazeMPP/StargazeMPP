// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";

/// @dev Bare-bones recipient with no logic — stands in for the real staker
///      reward pool. Treated by BurnController as a vanilla `transferFrom`
///      target; the actual reward-distribution machinery lives elsewhere.
contract MockStakerPool {}

contract BurnControllerIntegrationTest is Test {
    GAZEToken internal gaze;
    BurnController internal bc;
    MockStakerPool internal pool;

    address internal admin = address(0xA11CE);
    address internal router = address(0x7007);
    address internal staker = address(0x57A4);

    uint256 internal constant INITIAL_SUPPLY = 1_000_000e18;

    event RoutingFeeProcessed(address indexed payer, uint256 burned, uint256 toStakers);
    event TransferObserved(address indexed from, address indexed to, uint256 amount);
    event ReputationVote(address indexed voter, uint256 amount);

    function setUp() public {
        gaze = new GAZEToken(INITIAL_SUPPLY, admin);
        bc = new BurnController(address(gaze), admin);
        pool = new MockStakerPool();

        vm.startPrank(admin);
        gaze.setBurnController(address(bc));
        bc.setStakerPool(address(pool));
        bc.grantRole(bc.ROUTER_ROLE(), router);
        gaze.transfer(router, 10_000e18);
        gaze.transfer(staker, 1_000e18);
        vm.stopPrank();
    }

    function _approveAndRoute(uint256 fee) internal {
        vm.prank(router);
        gaze.approve(address(bc), fee);
        vm.prank(router);
        bc.processRoutingFee(fee);
    }

    function test_HappyPath_SplitsFiftyFifty() public {
        uint256 fee = 100e18;
        uint256 supplyBefore = gaze.totalSupply();
        uint256 routerBefore = gaze.balanceOf(router);

        vm.prank(router);
        gaze.approve(address(bc), fee);

        vm.expectEmit(true, false, false, true, address(bc));
        emit RoutingFeeProcessed(router, fee / 2, fee - fee / 2);
        vm.prank(router);
        bc.processRoutingFee(fee);

        assertEq(gaze.totalSupply(), supplyBefore - fee / 2, "supply burned by half");
        assertEq(gaze.balanceOf(address(pool)), fee - fee / 2, "pool got the other half");
        assertEq(gaze.balanceOf(router), routerBefore - fee, "router lost the full fee");
        assertEq(bc.totalBurned(), fee / 2);
        assertEq(bc.totalDistributedToStakers(), fee - fee / 2);
    }

    function test_OddAmount_RoundsBurnDown() public {
        uint256 fee = 7;
        _approveAndRoute(fee);

        assertEq(bc.totalBurned(), 3, "burns floor(7/2)=3");
        assertEq(bc.totalDistributedToStakers(), 4, "pool receives 7-3=4");
        assertEq(gaze.balanceOf(address(pool)), 4);
    }

    function test_Aggregates_AcrossMultipleFees() public {
        _approveAndRoute(40e18);
        _approveAndRoute(60e18);
        _approveAndRoute(100e18);

        // 20 + 30 + 50 = 100 burned, same to stakers.
        assertEq(bc.totalBurned(), 100e18);
        assertEq(bc.totalDistributedToStakers(), 100e18);
        assertEq(gaze.balanceOf(address(pool)), 100e18);
    }

    function test_TransferHook_FiresOnStakerPoolLegOnly() public {
        // The 50% burn calls `_update(router, address(0), ...)` which the hook
        // skips (to == 0). Only the staker-pool transfer should emit.
        vm.prank(router);
        gaze.approve(address(bc), 100e18);

        vm.expectEmit(true, true, false, true, address(bc));
        emit TransferObserved(router, address(pool), 50e18);
        vm.prank(router);
        bc.processRoutingFee(100e18);
    }

    function test_Reverts_OnZeroAmount() public {
        vm.prank(router);
        vm.expectRevert(BurnController.ZeroAmount.selector);
        bc.processRoutingFee(0);
    }

    function test_Reverts_WhenStakerPoolUnset() public {
        BurnController fresh = new BurnController(address(gaze), admin);
        bytes32 routerRole = fresh.ROUTER_ROLE();
        vm.prank(admin);
        fresh.grantRole(routerRole, router);

        vm.prank(router);
        gaze.approve(address(fresh), 10e18);
        vm.prank(router);
        vm.expectRevert(BurnController.StakerPoolUnset.selector);
        fresh.processRoutingFee(10e18);
    }

    function test_Reverts_OnNonRouterCaller() public {
        bytes32 routerRole = bc.ROUTER_ROLE();
        vm.prank(staker);
        gaze.approve(address(bc), 10e18);
        vm.prank(staker);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, staker, routerRole)
        );
        bc.processRoutingFee(10e18);
    }

    function test_Reverts_WhenRouterDidNotApprove() public {
        // No approve() call before processRoutingFee.
        vm.prank(router);
        vm.expectRevert();
        bc.processRoutingFee(50e18);
    }

    function test_BurnForReputationVoteFrom_OnlyRegistryRole() public {
        address registry = address(0xBEEF); // stand-in registry caller
        bytes32 registryRole = bc.REGISTRY_ROLE();

        // Voter funds and approves the controller for the vote burn.
        uint256 burnAmount = bc.REPUTATION_VOTE_BURN_AMOUNT();
        vm.prank(admin);
        gaze.transfer(staker, burnAmount);
        vm.prank(staker);
        gaze.approve(address(bc), burnAmount);

        // Without the role: revert.
        vm.prank(registry);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, registry, registryRole)
        );
        bc.burnForReputationVoteFrom(staker);

        // Grant the role and retry.
        vm.prank(admin);
        bc.grantRole(registryRole, registry);

        uint256 supplyBefore = gaze.totalSupply();
        uint256 burnedBefore = bc.totalBurned();
        uint256 voterBefore = gaze.balanceOf(staker);

        vm.expectEmit(true, false, false, true, address(bc));
        emit ReputationVote(staker, burnAmount);
        vm.prank(registry);
        bc.burnForReputationVoteFrom(staker);

        assertEq(gaze.balanceOf(staker), voterBefore - burnAmount, "voter debited");
        assertEq(gaze.totalSupply(), supplyBefore - burnAmount, "supply reduced");
        assertEq(bc.totalBurned(), burnedBefore + burnAmount, "totalBurned accumulates");
    }

    function testFuzz_AlwaysSumsToFee(uint96 rawFee) public {
        uint256 fee = uint256(bound(rawFee, 1, 1_000e18));
        _approveAndRoute(fee);
        assertEq(bc.totalBurned() + bc.totalDistributedToStakers(), fee);
        assertEq(gaze.balanceOf(address(pool)), fee - fee / 2);
    }
}
