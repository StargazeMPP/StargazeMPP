// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";

contract FuzzMockUSD is ERC20 {
    constructor() ERC20("PathUSD", "PUSD") {}
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @dev Property-based tests around the cumulative voucher schema:
///         monotonicity, spending-limit cap, refund accounting.
contract StargazeEscrowFuzzTest is Test {
    StargazeEscrow internal escrow;
    FuzzMockUSD internal pathUsd;
    address internal admin = address(0xA11CE);
    address internal router = address(0x4001);
    address internal agent;
    uint256 internal agentKey;
    address internal provider = address(0x7E57);
    address internal feeSink = address(0xFEE);

    function setUp() public {
        (agent, agentKey) = makeAddrAndKey("agent-fuzz");

        pathUsd = new FuzzMockUSD();
        escrow = new StargazeEscrow(address(pathUsd), admin);

        vm.startPrank(admin);
        escrow.grantRole(escrow.ROUTER_ROLE(), router);
        escrow.setRoutingFeeSink(feeSink);
        vm.stopPrank();

        pathUsd.mint(agent, 100_000_000e6);
        vm.prank(agent);
        pathUsd.approve(address(escrow), type(uint256).max);
    }

    function _signVoucher(bytes32 sessionId, uint256 cumulative, uint256 nonce, uint64 expiry)
        internal
        view
        returns (bytes memory)
    {
        bytes32 structHash = keccak256(
            abi.encode(
                escrow.VOUCHER_TYPEHASH(),
                sessionId,
                agent,
                provider,
                cumulative,
                nonce,
                expiry
            )
        );
        bytes32 digest = keccak256(
            abi.encodePacked("\x19\x01", escrow.eip712DomainSeparator(), structHash)
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(agentKey, digest);
        return abi.encodePacked(r, s, v);
    }

    /// @notice For any deposit + cumulative spend within the spending limit,
    ///   the settlement must preserve the accounting identity
    ///   `deposit == providerPayout + routingFee + refundToAgent`.
    function testFuzz_RefundAccountingIdentity(uint128 depositRaw, uint128 cumulativeRaw) public {
        // Deposit within the agent's pre-minted balance, large enough that
        // a 2% routing fee leaves at least one wei of headroom on the spend.
        uint256 deposit = bound(uint256(depositRaw), 100, 100_000_000e6 - 1);
        // Reserve 5% headroom so `cumulative + cumulative*0.02 ≤ deposit`
        // — i.e. the routing fee transfer never tries to overdraw escrow.
        uint256 cumulative = bound(uint256(cumulativeRaw), 1, (deposit * 9500) / 10_000);

        bytes32 sessionId = keccak256(abi.encode("session", deposit, cumulative));
        uint64 expiresAt = uint64(block.timestamp + 1 hours);
        uint256 limit = deposit;

        uint256 agentStart = pathUsd.balanceOf(agent);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, limit, expiresAt);

        uint64 voucherExpiry = uint64(block.timestamp + 1 hours);
        bytes memory sig = _signVoucher(sessionId, cumulative, 1, voucherExpiry);

        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: cumulative,
            nonce: 1,
            expiry: voucherExpiry,
            signature: sig
        });

        vm.prank(router);
        escrow.settle(sessionId, claims);

        uint256 fee = (cumulative * escrow.ROUTING_FEE_BPS()) / escrow.BPS_DENOMINATOR();
        uint256 refund = deposit - cumulative - fee;

        assertEq(pathUsd.balanceOf(provider), cumulative, "provider payout");
        assertEq(pathUsd.balanceOf(feeSink), fee, "routing fee");
        assertEq(pathUsd.balanceOf(agent), agentStart - cumulative - fee, "agent net");
        assertEq(pathUsd.balanceOf(address(escrow)), 0, "escrow drained");
        assertEq(deposit, cumulative + fee + refund, "accounting identity");
    }

    /// @notice A single voucher whose cumulative amount exceeds `spendingLimit`
    ///   must revert with `SpendingLimitExceeded` — no partial settlement.
    function testFuzz_SpendingLimitCap(uint128 depositRaw, uint128 cumulativeOverflow) public {
        uint256 deposit = bound(uint256(depositRaw), 1_000_000, 100_000_000e6 - 1);
        uint256 limit = deposit / 2;
        // Pick `cumulative` strictly above the limit but ≤ deposit so the
        // settlement fails on the limit check rather than insufficient
        // escrow.
        uint256 cumulative = bound(uint256(cumulativeOverflow), limit + 1, deposit);

        bytes32 sessionId = keccak256(abi.encode("over-limit", deposit, cumulative));
        uint64 expiresAt = uint64(block.timestamp + 1 hours);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, limit, expiresAt);

        uint64 voucherExpiry = uint64(block.timestamp + 1 hours);
        bytes memory sig = _signVoucher(sessionId, cumulative, 1, voucherExpiry);

        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: cumulative,
            nonce: 1,
            expiry: voucherExpiry,
            signature: sig
        });

        vm.prank(router);
        vm.expectRevert(StargazeEscrow.SpendingLimitExceeded.selector);
        escrow.settle(sessionId, claims);

        assertEq(pathUsd.balanceOf(provider), 0, "no partial payout");
    }

    /// @notice The same voucher cannot be settled twice — even across separate
    ///   `settle` calls — because `consumedVouchers` is a permanent hashmap
    ///   keyed on the typed-data digest.
    function testFuzz_VoucherReplayBlocked(uint128 amountRaw) public {
        // Cap so the 4× deposit headroom fits within the agent's mint.
        uint256 amount = bound(uint256(amountRaw), 1_000_000, 25_000_000e6 - 1);
        uint256 deposit = amount * 4;

        bytes32 sessionId = keccak256(abi.encode("replay", amount));
        uint64 expiresAt = uint64(block.timestamp + 1 hours);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, deposit, expiresAt);

        uint64 voucherExpiry = uint64(block.timestamp + 1 hours);
        bytes memory sig = _signVoucher(sessionId, amount, 1, voucherExpiry);

        StargazeEscrow.VoucherClaim memory claim = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: amount,
            nonce: 1,
            expiry: voucherExpiry,
            signature: sig
        });
        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = claim;

        vm.prank(router);
        escrow.settle(sessionId, claims);

        // Session is now closed — a second settle attempt reverts `AlreadySettled`.
        vm.prank(router);
        vm.expectRevert(StargazeEscrow.AlreadySettled.selector);
        escrow.settle(sessionId, claims);
    }

    /// @notice A voucher whose signer differs from `session.agentWallet`
    ///   must be rejected — even if the cumulative amount and provider
    ///   are otherwise valid.
    function testFuzz_BadSignerRejected(uint128 amountRaw) public {
        uint256 amount = bound(uint256(amountRaw), 1, 1_000_000e6 - 1);
        uint256 deposit = amount * 2;

        bytes32 sessionId = keccak256(abi.encode("bad-signer", amount));
        uint64 expiresAt = uint64(block.timestamp + 1 hours);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, deposit, expiresAt);

        // Sign with a different key entirely.
        (, uint256 attackerKey) = makeAddrAndKey("attacker");
        bytes32 structHash = keccak256(
            abi.encode(
                escrow.VOUCHER_TYPEHASH(),
                sessionId,
                agent,
                provider,
                amount,
                uint256(1),
                uint64(block.timestamp + 1 hours)
            )
        );
        bytes32 digest = keccak256(
            abi.encodePacked("\x19\x01", escrow.eip712DomainSeparator(), structHash)
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(attackerKey, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: amount,
            nonce: 1,
            expiry: uint64(block.timestamp + 1 hours),
            signature: sig
        });

        vm.prank(router);
        vm.expectRevert(StargazeEscrow.BadSignature.selector);
        escrow.settle(sessionId, claims);
    }
}
