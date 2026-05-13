// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test, Vm} from "forge-std/Test.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";
import {BurnController} from "../src/BurnController.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {MockUSD} from "./StargazeEscrow.t.sol";
import {MockStakerPool} from "./BurnControllerIntegration.t.sol";

/// @title EscrowBurnLoopTest
/// @notice End-to-end proof of the StargazeMPP routing-fee loop. An agent
///         opens a PathUSD-funded session; the router settles a signed
///         voucher, which routes a 2% PathUSD fee to a settler EOA. The
///         settler (here, the test acting as the off-chain bot) converts
///         that fee to GAZE at a fixed test rate and calls
///         BurnController.processRoutingFee, which burns 50% and forwards
///         50% to the staker pool.
contract EscrowBurnLoopTest is Test {
    StargazeEscrow internal escrow;
    BurnController internal burnController;
    GAZEToken internal gaze;
    MockUSD internal pathUsd;
    MockStakerPool internal stakerPool;

    address internal admin = makeAddr("admin");
    address internal router = makeAddr("router");
    address internal settler = makeAddr("settler");
    address internal provider = makeAddr("provider");
    address internal agent;
    uint256 internal agentKey;

    uint256 internal constant INITIAL_GAZE_SUPPLY = 1_000_000e18;
    /// @dev Test-only PathUSD-to-GAZE rate. Production reads from an oracle.
    ///      1 PathUSD (1e6 base units) => 1 GAZE (1e18 base units) ⇒ rate = 1e12.
    uint256 internal constant PATH_USD_TO_GAZE_RATE = 1e12;

    function setUp() public {
        (agent, agentKey) = makeAddrAndKey("agent");

        pathUsd = new MockUSD();
        gaze = new GAZEToken(INITIAL_GAZE_SUPPLY, admin);
        burnController = new BurnController(address(gaze), admin);
        escrow = new StargazeEscrow(address(pathUsd), admin);
        stakerPool = new MockStakerPool();

        vm.startPrank(admin);
        escrow.grantRole(escrow.ROUTER_ROLE(), router);
        escrow.setRoutingFeeSink(settler);
        burnController.grantRole(burnController.ROUTER_ROLE(), settler);
        burnController.setStakerPool(address(stakerPool));
        gaze.setBurnController(address(burnController));
        gaze.transfer(settler, 10_000e18);
        vm.stopPrank();

        vm.prank(settler);
        gaze.approve(address(burnController), type(uint256).max);

        pathUsd.mint(agent, 1_000e6);
        vm.prank(agent);
        pathUsd.approve(address(escrow), type(uint256).max);
    }

    /// @notice Full StargazeMPP fee loop:
    ///         agent opens session → router settles voucher → 2% PathUSD fee
    ///         lands at settler → settler converts to GAZE → BurnController
    ///         splits 50/50 between burn and staker pool. Verifies PathUSD
    ///         provider/agent/settler balances and GAZE supply/staker delta.
    function test_VoucherRedeem_RoutesFeeToBurnController_5050Split() external {
        bytes32 sessionId = bytes32(uint256(0xBEEF));
        uint256 deposit = 100e6;
        uint256 agentMaxSpend = 80e6;
        uint64 sessionExpiry = uint64(block.timestamp + 1 days);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, agentMaxSpend, sessionExpiry);

        uint256 cumulativeSpend = 50e6;
        uint256 nonce = 1;
        uint64 voucherExpiry = uint64(block.timestamp + 1 hours);

        bytes memory sig = _signVoucher(sessionId, agent, provider, cumulativeSpend, nonce, voucherExpiry);

        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: cumulativeSpend,
            nonce: nonce,
            expiry: voucherExpiry,
            signature: sig
        });

        vm.recordLogs();
        vm.prank(router);
        escrow.settle(sessionId, claims);

        uint256 emittedRoutingFee = _decodeRoutingFeeFromLogs(sessionId);
        uint256 expectedRoutingFee = (cumulativeSpend * 200) / 10_000;
        assertEq(emittedRoutingFee, expectedRoutingFee, "SessionSettled.routingFee mismatch");

        // --- PathUSD side ---
        assertEq(pathUsd.balanceOf(provider), cumulativeSpend, "provider got cumulativeSpend");
        assertEq(pathUsd.balanceOf(settler), expectedRoutingFee, "settler got 2% routing fee");
        uint256 expectedAgentBalance = 1_000e6 - deposit + (deposit - cumulativeSpend - expectedRoutingFee);
        assertEq(pathUsd.balanceOf(agent), expectedAgentBalance, "agent refund correct");

        // --- Simulate off-chain settler bot: convert PathUSD fee → GAZE ---
        uint256 gazeFee = expectedRoutingFee * PATH_USD_TO_GAZE_RATE; // 1e6 * 1e12 = 1e18
        assertEq(gazeFee, 1e18, "test math sanity: gazeFee = 1 GAZE");

        uint256 supplyBefore = gaze.totalSupply();
        uint256 burnedBefore = burnController.totalBurned();
        uint256 stakersBefore = burnController.totalDistributedToStakers();

        vm.prank(settler);
        burnController.processRoutingFee(gazeFee);

        // --- GAZE side ---
        uint256 expectedBurn = gazeFee / 2;
        uint256 expectedToStakers = gazeFee - expectedBurn;
        assertEq(gaze.totalSupply(), supplyBefore - expectedBurn, "supply burned by half");
        assertEq(gaze.balanceOf(address(stakerPool)), expectedToStakers, "staker pool got the other half");
        assertEq(burnController.totalBurned(), burnedBefore + expectedBurn, "totalBurned accumulated");
        assertEq(
            burnController.totalDistributedToStakers(),
            stakersBefore + expectedToStakers,
            "totalDistributedToStakers accumulated"
        );
    }

    function _signVoucher(
        bytes32 sessionId,
        address agentWallet,
        address voucherProvider,
        uint256 cumulativeAmount,
        uint256 nonce,
        uint64 expiry
    ) internal view returns (bytes memory) {
        bytes32 structHash = keccak256(
            abi.encode(
                escrow.VOUCHER_TYPEHASH(),
                sessionId,
                agentWallet,
                voucherProvider,
                cumulativeAmount,
                nonce,
                expiry
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", escrow.eip712DomainSeparator(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(agentKey, digest);
        return abi.encodePacked(r, s, v);
    }

    function _decodeRoutingFeeFromLogs(bytes32 sessionId) internal returns (uint256 routingFee) {
        // SessionSettled(bytes32 indexed sessionId, uint256 totalToProviders, uint256 routingFee, uint256 refundToAgent)
        bytes32 sig = keccak256("SessionSettled(bytes32,uint256,uint256,uint256)");
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics.length >= 2 && entries[i].topics[0] == sig && entries[i].topics[1] == sessionId) {
                (, routingFee, ) = abi.decode(entries[i].data, (uint256, uint256, uint256));
                return routingFee;
            }
        }
        revert("SessionSettled event not found");
    }
}
