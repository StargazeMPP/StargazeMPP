// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {StargazeStakeMirror, Any2EVMMessage, EVMTokenAmount} from "../src/StargazeStakeMirror.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";

contract StargazeStakeMirrorTest is Test {
    StargazeStakeMirror internal mirror;

    address internal admin = address(0xA11CE);
    address internal ccipRouter = address(0xCC1F);
    address internal stakerA = address(0xA1);
    address internal stakerB = address(0xB2);

    /// @dev Chainlink's published Solana mainnet selector.
    uint64 internal constant SOLANA_SELECTOR = 5854466;
    bytes internal anchorSender = abi.encode(bytes32("StargazeAnchor111111111111111111"));

    /// @dev Default verified threshold: 500 GAZE at 6 decimals, matching the
    ///      Solana side's `VERIFIED_STAKE_DEFAULT` constant.
    uint256 internal constant DEFAULT_THRESHOLD = 500_000_000;

    event SourceAllowed(uint64 indexed selector, bool allowed);
    event SenderAllowed(uint64 indexed selector, bytes32 indexed senderHash, bool allowed);
    event StakeMirrored(
        bytes32 indexed providerId, address indexed owner, uint256 amount, uint256 totalStake, bytes32 messageId
    );
    event VerifiedThresholdSet(uint256 previous, uint256 current);

    function setUp() public {
        mirror = new StargazeStakeMirror(admin, DEFAULT_THRESHOLD);

        vm.startPrank(admin);
        mirror.grantRole(mirror.CCIP_ROUTER_ROLE(), ccipRouter);
        mirror.setAllowedSource(SOLANA_SELECTOR, true);
        mirror.setAllowedSender(SOLANA_SELECTOR, anchorSender, true);
        vm.stopPrank();
    }

    function _msg(bytes32 providerId, address owner, uint256 amount, bytes memory sender, uint64 selector)
        internal
        pure
        returns (Any2EVMMessage memory m)
    {
        m.messageId = keccak256(abi.encode("msg", providerId, owner, amount));
        m.sourceChainSelector = selector;
        m.sender = sender;
        m.data = abi.encode(providerId, owner, amount);
        m.destTokenAmounts = new EVMTokenAmount[](0);
    }

    function _deliver(bytes32 providerId, address owner, uint256 amount) internal {
        Any2EVMMessage memory message = _msg(providerId, owner, amount, anchorSender, SOLANA_SELECTOR);
        vm.prank(ccipRouter);
        mirror.ccipReceive(message);
    }

    function test_HappyPath_StoresMirrorAndAggregates() public {
        bytes32 providerId = keccak256("mpp32");
        uint256 amount = DEFAULT_THRESHOLD;

        _deliver(providerId, stakerA, amount);

        assertEq(mirror.stakeOf(providerId, stakerA), amount, "stakeOf stored");
        assertEq(mirror.totalStake(providerId), amount, "totalStake aggregate");
        assertTrue(mirror.isVerifiedStake(providerId), "at threshold -> verified");

        // Drop the staker below the threshold and re-check.
        _deliver(providerId, stakerA, amount - 1);
        assertFalse(mirror.isVerifiedStake(providerId), "below threshold -> not verified");
    }

    function test_TwoOwnersAggregate() public {
        bytes32 providerId = keccak256("mpp32");

        _deliver(providerId, stakerA, 200_000_000);
        _deliver(providerId, stakerB, 300_000_000);

        assertEq(mirror.stakeOf(providerId, stakerA), 200_000_000, "stakerA mirror");
        assertEq(mirror.stakeOf(providerId, stakerB), 300_000_000, "stakerB mirror");
        assertEq(mirror.totalStake(providerId), 500_000_000, "total = sum of both");
    }

    function test_ResendUpdatesNotAdds() public {
        bytes32 providerId = keccak256("mpp32");

        _deliver(providerId, stakerA, 100);
        _deliver(providerId, stakerA, 300);

        assertEq(mirror.stakeOf(providerId, stakerA), 300, "latest snapshot stored");
        assertEq(mirror.totalStake(providerId), 300, "aggregate is snapshot, not sum");
    }

    function test_DecrementOnPartialUnstake() public {
        bytes32 providerId = keccak256("mpp32");

        _deliver(providerId, stakerA, 500);
        _deliver(providerId, stakerA, 200);

        assertEq(mirror.stakeOf(providerId, stakerA), 200, "decremented mirror");
        assertEq(mirror.totalStake(providerId), 200, "decremented aggregate");
    }

    function test_VerifiedThresholdConfigurable() public {
        bytes32 providerId = keccak256("mpp32");

        _deliver(providerId, stakerA, DEFAULT_THRESHOLD);
        assertTrue(mirror.isVerifiedStake(providerId), "verified at default threshold");

        // Admin raises the bar.
        uint256 newThreshold = DEFAULT_THRESHOLD * 2;
        vm.expectEmit(false, false, false, true, address(mirror));
        emit VerifiedThresholdSet(DEFAULT_THRESHOLD, newThreshold);
        vm.prank(admin);
        mirror.setVerifiedThreshold(newThreshold);

        assertEq(mirror.verifiedThreshold(), newThreshold, "threshold updated");
        assertFalse(mirror.isVerifiedStake(providerId), "previously-verified now drops");
    }

    function test_IsVerifiedStake_FalseByDefault() public view {
        bytes32 ghost = keccak256("unknown-provider");
        assertFalse(mirror.isVerifiedStake(ghost), "unknown providerId -> false");
    }

    function test_RejectsUnknownSource() public {
        Any2EVMMessage memory message = _msg(keccak256("prov"), stakerA, 100, anchorSender, 99_999);

        vm.prank(ccipRouter);
        vm.expectRevert(abi.encodeWithSelector(StargazeStakeMirror.SourceNotAllowed.selector, uint64(99_999)));
        mirror.ccipReceive(message);
    }

    function test_RejectsUnknownSender() public {
        bytes memory impostor = abi.encode(bytes32("imposter11111111111"));
        Any2EVMMessage memory message = _msg(keccak256("prov"), stakerA, 100, impostor, SOLANA_SELECTOR);

        vm.prank(ccipRouter);
        vm.expectRevert(abi.encodeWithSelector(StargazeStakeMirror.SenderNotAllowed.selector, SOLANA_SELECTOR));
        mirror.ccipReceive(message);
    }

    function test_RejectsNonRouterCaller() public {
        Any2EVMMessage memory message = _msg(keccak256("prov"), stakerA, 100, anchorSender, SOLANA_SELECTOR);
        vm.prank(address(0xBEEF));
        vm.expectRevert();
        mirror.ccipReceive(message);
    }

    function test_SetVerifiedThreshold_AdminOnly() public {
        address stranger = address(0xDEAD0);
        bytes32 adminRole = mirror.DEFAULT_ADMIN_ROLE();

        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, stranger, adminRole)
        );
        mirror.setVerifiedThreshold(123);

        vm.expectEmit(false, false, false, true, address(mirror));
        emit VerifiedThresholdSet(DEFAULT_THRESHOLD, 123);
        vm.prank(admin);
        mirror.setVerifiedThreshold(123);
        assertEq(mirror.verifiedThreshold(), 123, "threshold updated by admin");
    }

    function testFuzz_AggregateUpdatesAreConsistent(uint128 a, uint128 b) public {
        bytes32 providerId = keccak256("fuzz-provider");

        _deliver(providerId, stakerA, uint256(a));
        _deliver(providerId, stakerA, uint256(b));

        assertEq(mirror.stakeOf(providerId, stakerA), uint256(b), "latest snapshot stored");
        assertEq(mirror.totalStake(providerId), uint256(b), "aggregate matches snapshot");
    }

    function test_Integration_AsStakeChecker() public {
        // 1. Stand up a fresh registry; we'll wire the mirror in as its checker.
        address oracle = address(0x07AC);
        bytes32 providerId = keccak256("provider-integration");

        StargazeRegistry registry = new StargazeRegistry(admin);

        vm.startPrank(admin);
        registry.grantRole(registry.ORACLE_ROLE(), oracle);
        registry.setStakeChecker(address(mirror));
        vm.stopPrank();

        // 2. Register the provider (anyone can call `register`).
        registry.register(providerId, keccak256("search"), bytes32(uint256(0xCAFE)));

        // 3. Raise reputation to clear the verified-score gate. Cache the
        //    threshold first: an external view as an argument would consume
        //    the prank cheatcode and leave the actual call untracked.
        uint256 verifiedScore = registry.VERIFIED_SCORE();
        vm.prank(oracle);
        registry.setReputationScore(providerId, verifiedScore);

        // Still not verified: zero stake mirrored.
        assertFalse(registry.isVerified(providerId), "no stake yet -> unverified");

        // 4. Push enough stake through CCIP to clear the mirror threshold.
        _deliver(providerId, stakerA, DEFAULT_THRESHOLD);
        assertTrue(registry.isVerified(providerId), "score + stake -> verified");

        // 5. Drop the stake below the threshold -> registry must report false.
        _deliver(providerId, stakerA, DEFAULT_THRESHOLD - 1);
        assertFalse(registry.isVerified(providerId), "stake fell below threshold -> unverified");
    }
}
