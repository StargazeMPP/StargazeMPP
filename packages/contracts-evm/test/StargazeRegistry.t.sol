// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {GAZEToken} from "../src/GAZEToken.sol";
import {BurnController} from "../src/BurnController.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";

contract StargazeRegistryTest is Test {
    GAZEToken internal gaze;
    BurnController internal bc;
    StargazeRegistry internal registry;

    address internal admin = address(0xA11CE);
    address internal provider = address(0xB0B);
    address internal voter = address(0xC0C);
    address internal slasher = address(0x51A5);
    address internal oracle = address(0x07AC);
    address internal stranger = address(0xDEAD0);

    bytes32 internal constant PROVIDER_ID = keccak256("provider-1");
    bytes32 internal constant CATEGORY = keccak256("search");
    bytes32 internal constant META_CID = bytes32(uint256(0xCAFE));

    uint256 internal constant INITIAL_SUPPLY = 1_000_000e18;

    // Event signatures duplicated for vm.expectEmit.
    event ProviderRegistered(bytes32 indexed providerId, address indexed owner, uint256 stake, bytes32 categoryHash);
    event ProviderUpdated(bytes32 indexed providerId, bytes32 metaCid);
    event StakeIncreased(bytes32 indexed providerId, uint256 added, uint256 newTotal);
    event ProviderSlashed(bytes32 indexed providerId, uint256 amount, uint256 remaining, string reason);
    event ReputationUpdated(bytes32 indexed providerId, uint256 score);

    function setUp() public {
        gaze = new GAZEToken(INITIAL_SUPPLY, admin);
        bc = new BurnController(address(gaze), admin);
        registry = new StargazeRegistry(address(gaze), address(bc), admin);

        vm.startPrank(admin);
        gaze.setBurnController(address(bc));
        gaze.transfer(provider, 10_000e18);
        gaze.transfer(voter, 1_000e18);
        vm.stopPrank();
    }

    function _register(address who, bytes32 id, uint256 stakeAmount) internal {
        vm.prank(who);
        gaze.approve(address(registry), stakeAmount);
        vm.prank(who);
        registry.register(id, CATEGORY, META_CID, stakeAmount);
    }

    function test_Register_HappyPath() public {
        uint256 stakeAmount = registry.MIN_STAKE();
        uint256 providerBefore = gaze.balanceOf(provider);

        vm.prank(provider);
        gaze.approve(address(registry), stakeAmount);

        vm.expectEmit(true, true, false, true, address(registry));
        emit ProviderRegistered(PROVIDER_ID, provider, stakeAmount, CATEGORY);
        vm.prank(provider);
        registry.register(PROVIDER_ID, CATEGORY, META_CID, stakeAmount);

        (
            address owner,
            uint256 stake,
            uint256 reputationScore,
            bool registered,
            bytes32 categoryHash,
            bytes32 metaCid
        ) = registry.providers(PROVIDER_ID);
        assertEq(owner, provider, "owner stored");
        assertEq(stake, stakeAmount, "stake stored");
        assertEq(reputationScore, 500, "reputation defaults to 500");
        assertTrue(registered, "registered flag");
        assertEq(categoryHash, CATEGORY, "category");
        assertEq(metaCid, META_CID, "metaCid");

        assertEq(gaze.balanceOf(provider), providerBefore - stakeAmount, "provider debited");
        assertEq(gaze.balanceOf(address(registry)), stakeAmount, "registry credited");
    }

    function test_Register_RevertsOnLowStake() public {
        uint256 tooLow = registry.MIN_STAKE() - 1;
        vm.prank(provider);
        gaze.approve(address(registry), tooLow);

        vm.prank(provider);
        vm.expectRevert(StargazeRegistry.StakeTooLow.selector);
        registry.register(PROVIDER_ID, CATEGORY, META_CID, tooLow);
    }

    function test_Register_RevertsOnDuplicate() public {
        uint256 stakeAmount = registry.MIN_STAKE();
        _register(provider, PROVIDER_ID, stakeAmount);

        vm.prank(provider);
        gaze.approve(address(registry), stakeAmount);
        vm.prank(provider);
        vm.expectRevert(StargazeRegistry.AlreadyRegistered.selector);
        registry.register(PROVIDER_ID, CATEGORY, META_CID, stakeAmount);
    }

    function test_Register_RevertsWithoutApproval() public {
        uint256 stakeAmount = registry.MIN_STAKE();
        vm.prank(provider);
        vm.expectRevert(); // SafeERC20 surfaces an ERC20 allowance failure.
        registry.register(PROVIDER_ID, CATEGORY, META_CID, stakeAmount);
    }

    function test_UpdateMeta_OnlyOwner() public {
        _register(provider, PROVIDER_ID, registry.MIN_STAKE());

        bytes32 newCid = bytes32(uint256(0xBEEF));

        vm.prank(stranger);
        vm.expectRevert(StargazeRegistry.NotProviderOwner.selector);
        registry.updateMeta(PROVIDER_ID, newCid);

        vm.expectEmit(true, false, false, true, address(registry));
        emit ProviderUpdated(PROVIDER_ID, newCid);
        vm.prank(provider);
        registry.updateMeta(PROVIDER_ID, newCid);

        (,,,,, bytes32 metaCid) = registry.providers(PROVIDER_ID);
        assertEq(metaCid, newCid, "metaCid updated");
    }

    function test_UpdateMeta_RevertsWhenNotRegistered() public {
        vm.prank(provider);
        vm.expectRevert(StargazeRegistry.NotRegistered.selector);
        registry.updateMeta(keccak256("ghost"), bytes32(uint256(1)));
    }

    function test_IncreaseStake_AccumulatesAndEmits() public {
        uint256 initialStake = registry.MIN_STAKE();
        _register(provider, PROVIDER_ID, initialStake);

        uint256 addAmount = 25e18;
        vm.prank(provider);
        gaze.approve(address(registry), addAmount);

        vm.expectEmit(true, false, false, true, address(registry));
        emit StakeIncreased(PROVIDER_ID, addAmount, initialStake + addAmount);
        vm.prank(provider);
        registry.increaseStake(PROVIDER_ID, addAmount);

        (, uint256 stake,,,,) = registry.providers(PROVIDER_ID);
        assertEq(stake, initialStake + addAmount, "stake accumulated");
    }

    function test_Slash_OnlySlasherRole() public {
        uint256 initialStake = registry.MIN_STAKE();
        _register(provider, PROVIDER_ID, initialStake);

        bytes32 slasherRole = registry.SLASHER_ROLE();

        // No role yet.
        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, stranger, slasherRole)
        );
        registry.slash(PROVIDER_ID, 10e18, "bad-behavior");

        vm.prank(admin);
        registry.grantRole(slasherRole, slasher);

        uint256 deadBefore = gaze.balanceOf(address(0xdead));
        uint256 slashAmount = 10e18;

        vm.expectEmit(true, false, false, true, address(registry));
        emit ProviderSlashed(PROVIDER_ID, slashAmount, initialStake - slashAmount, "bad-behavior");
        vm.prank(slasher);
        registry.slash(PROVIDER_ID, slashAmount, "bad-behavior");

        (, uint256 stake,,,,) = registry.providers(PROVIDER_ID);
        assertEq(stake, initialStake - slashAmount, "stake reduced");
        assertEq(gaze.balanceOf(address(0xdead)), deadBefore + slashAmount, "0xdead got the slashed amount");
    }

    function test_Slash_CapsAtAvailableStake() public {
        uint256 initialStake = registry.MIN_STAKE();
        _register(provider, PROVIDER_ID, initialStake);

        bytes32 slasherRole = registry.SLASHER_ROLE();
        vm.prank(admin);
        registry.grantRole(slasherRole, slasher);

        uint256 deadBefore = gaze.balanceOf(address(0xdead));
        uint256 requested = initialStake * 10; // way more than what's staked

        // Event records `actual` (the capped amount), not the requested amount.
        vm.expectEmit(true, false, false, true, address(registry));
        emit ProviderSlashed(PROVIDER_ID, initialStake, 0, "over-slash");
        vm.prank(slasher);
        registry.slash(PROVIDER_ID, requested, "over-slash");

        (, uint256 stake,,,,) = registry.providers(PROVIDER_ID);
        assertEq(stake, 0, "stake fully drained");
        assertEq(gaze.balanceOf(address(0xdead)), deadBefore + initialStake, "0xdead got only the available stake");
    }

    function test_SetReputationScore_OnlyOracle() public {
        _register(provider, PROVIDER_ID, registry.MIN_STAKE());

        bytes32 oracleRole = registry.ORACLE_ROLE();

        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, stranger, oracleRole)
        );
        registry.setReputationScore(PROVIDER_ID, 750);

        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        vm.expectEmit(true, false, false, true, address(registry));
        emit ReputationUpdated(PROVIDER_ID, 750);
        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, 750);

        (,, uint256 reputationScore,,,) = registry.providers(PROVIDER_ID);
        assertEq(reputationScore, 750, "score stored");
    }

    function test_SetReputationScore_RevertsOutOfRange() public {
        _register(provider, PROVIDER_ID, registry.MIN_STAKE());

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        // Cache the constant before prank/expectRevert: an external view as an
        // argument would consume the cheats and leave the actual call untracked.
        uint256 outOfRange = registry.MAX_REPUTATION() + 1;

        vm.prank(oracle);
        vm.expectRevert(StargazeRegistry.ScoreOutOfRange.selector);
        registry.setReputationScore(PROVIDER_ID, outOfRange);
    }

    function test_SetReputationScore_RevertsWhenNotRegistered() public {
        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        vm.prank(oracle);
        vm.expectRevert(StargazeRegistry.NotRegistered.selector);
        registry.setReputationScore(keccak256("ghost"), 600);
    }

    function test_CastReputationVote_KnownReverts() public {
        // FIXME: requires `burnForReputationVoteFrom(address voter)` on BurnController gated by REGISTRY_ROLE.
        _register(provider, PROVIDER_ID, registry.MIN_STAKE());

        vm.prank(voter);
        vm.expectRevert();
        registry.castReputationVote(PROVIDER_ID, true);
    }

    function test_CastReputationVote_RevertsWhenNotRegistered() public {
        vm.prank(voter);
        vm.expectRevert(StargazeRegistry.NotRegistered.selector);
        registry.castReputationVote(keccak256("ghost"), true);
    }

    function test_IsVerified_TransitionAcrossThresholds() public {
        uint256 verifiedStake = registry.VERIFIED_STAKE();
        uint256 verifiedScore = registry.VERIFIED_SCORE();
        uint256 belowVerified = verifiedStake - 1;

        _register(provider, PROVIDER_ID, belowVerified);
        assertFalse(registry.isVerified(PROVIDER_ID), "below stake threshold");

        // Top up stake to >= VERIFIED_STAKE. Default score is 500 < 800.
        uint256 topUp = 2; // belowVerified + 2 > verifiedStake
        vm.prank(provider);
        gaze.approve(address(registry), topUp);
        vm.prank(provider);
        registry.increaseStake(PROVIDER_ID, topUp);
        assertFalse(registry.isVerified(PROVIDER_ID), "stake ok but score still 500");

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, verifiedScore);
        assertTrue(registry.isVerified(PROVIDER_ID), "score == 800 flips verified");

        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, verifiedScore - 1);
        assertFalse(registry.isVerified(PROVIDER_ID), "score < 800 unverifies");
    }

    function testFuzz_ReputationScoreRoundTrip(uint16 raw) public {
        uint256 score = bound(uint256(raw), 0, registry.MAX_REPUTATION());

        _register(provider, PROVIDER_ID, registry.MIN_STAKE());

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, score);

        (,, uint256 stored,,,) = registry.providers(PROVIDER_ID);
        assertEq(stored, score, "round trip");
    }
}
