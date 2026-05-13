// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {StargazeRegistry} from "../src/StargazeRegistry.sol";
import {IStakeChecker} from "../src/IStakeChecker.sol";

contract TrueStakeChecker is IStakeChecker {
    function isVerifiedStake(bytes32) external pure returns (bool) {
        return true;
    }
}

contract FalseStakeChecker is IStakeChecker {
    function isVerifiedStake(bytes32) external pure returns (bool) {
        return false;
    }
}

contract StargazeRegistryTest is Test {
    StargazeRegistry internal registry;

    address internal admin = address(0xA11CE);
    address internal provider = address(0xB0B);
    address internal voter = address(0xC0C);
    address internal oracle = address(0x07AC);
    address internal stranger = address(0xDEAD0);

    bytes32 internal constant PROVIDER_ID = keccak256("provider-1");
    bytes32 internal constant CATEGORY = keccak256("search");
    bytes32 internal constant META_CID = bytes32(uint256(0xCAFE));

    // Event signatures duplicated for vm.expectEmit.
    event ProviderRegistered(bytes32 indexed providerId, address indexed owner, bytes32 categoryHash);
    event ProviderUpdated(bytes32 indexed providerId, bytes32 metaCid);
    event ReputationUpdated(bytes32 indexed providerId, uint256 score);
    event ReputationVoted(bytes32 indexed providerId, address indexed voter, bool accurate);
    event StakeCheckerSet(address indexed previous, address indexed current);

    function setUp() public {
        registry = new StargazeRegistry(admin);
    }

    function _register(address who, bytes32 id) internal {
        vm.prank(who);
        registry.register(id, CATEGORY, META_CID);
    }

    function test_Register_HappyPath() public {
        vm.expectEmit(true, true, false, true, address(registry));
        emit ProviderRegistered(PROVIDER_ID, provider, CATEGORY);
        vm.prank(provider);
        registry.register(PROVIDER_ID, CATEGORY, META_CID);

        (
            address owner,
            uint256 reputationScore,
            bool registered,
            bytes32 categoryHash,
            bytes32 metaCid
        ) = registry.providers(PROVIDER_ID);
        assertEq(owner, provider, "owner stored");
        assertEq(reputationScore, 500, "reputation defaults to 500");
        assertTrue(registered, "registered flag");
        assertEq(categoryHash, CATEGORY, "category");
        assertEq(metaCid, META_CID, "metaCid");
    }

    function test_Register_RevertsOnDuplicate() public {
        _register(provider, PROVIDER_ID);

        vm.prank(provider);
        vm.expectRevert(StargazeRegistry.AlreadyRegistered.selector);
        registry.register(PROVIDER_ID, CATEGORY, META_CID);
    }

    function test_UpdateMeta_OnlyOwner() public {
        _register(provider, PROVIDER_ID);

        bytes32 newCid = bytes32(uint256(0xBEEF));

        vm.prank(stranger);
        vm.expectRevert(StargazeRegistry.NotProviderOwner.selector);
        registry.updateMeta(PROVIDER_ID, newCid);

        vm.expectEmit(true, false, false, true, address(registry));
        emit ProviderUpdated(PROVIDER_ID, newCid);
        vm.prank(provider);
        registry.updateMeta(PROVIDER_ID, newCid);

        (,,,, bytes32 metaCid) = registry.providers(PROVIDER_ID);
        assertEq(metaCid, newCid, "metaCid updated");
    }

    function test_UpdateMeta_RevertsWhenNotRegistered() public {
        vm.prank(provider);
        vm.expectRevert(StargazeRegistry.NotRegistered.selector);
        registry.updateMeta(keccak256("ghost"), bytes32(uint256(1)));
    }

    function test_SetReputationScore_OnlyOracle() public {
        _register(provider, PROVIDER_ID);

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

        (, uint256 reputationScore,,,) = registry.providers(PROVIDER_ID);
        assertEq(reputationScore, 750, "score stored");
    }

    function test_SetReputationScore_RevertsOutOfRange() public {
        _register(provider, PROVIDER_ID);

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        // Cache the constant before prank/expectRevert: an external view as
        // an argument would consume the cheats and leave the actual call
        // untracked.
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

    function test_CastReputationVote_HappyPath() public {
        _register(provider, PROVIDER_ID);

        vm.expectEmit(true, true, false, true, address(registry));
        emit ReputationVoted(PROVIDER_ID, voter, true);
        vm.prank(voter);
        registry.castReputationVote(PROVIDER_ID, true);
    }

    function test_CastReputationVote_RevertsWhenNotRegistered() public {
        vm.prank(voter);
        vm.expectRevert(StargazeRegistry.NotRegistered.selector);
        registry.castReputationVote(keccak256("ghost"), true);
    }

    function test_SetStakeChecker_OnlyAdmin() public {
        TrueStakeChecker checker = new TrueStakeChecker();

        bytes32 adminRole = registry.DEFAULT_ADMIN_ROLE();
        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, stranger, adminRole)
        );
        registry.setStakeChecker(address(checker));

        vm.expectEmit(true, true, false, false, address(registry));
        emit StakeCheckerSet(address(0), address(checker));
        vm.prank(admin);
        registry.setStakeChecker(address(checker));
        assertEq(address(registry.stakeChecker()), address(checker), "checker stored");
    }

    function test_IsVerified_RequiresCheckerAndScore() public {
        _register(provider, PROVIDER_ID);

        // No checker wired yet -> always false.
        assertFalse(registry.isVerified(PROVIDER_ID), "no checker, unverified");

        TrueStakeChecker yes = new TrueStakeChecker();
        FalseStakeChecker no = new FalseStakeChecker();

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        // Score below threshold even with a permissive checker.
        vm.prank(admin);
        registry.setStakeChecker(address(yes));
        assertFalse(registry.isVerified(PROVIDER_ID), "score 500 < 800");

        // Bump score past the verified threshold.
        uint256 verifiedScore = registry.VERIFIED_SCORE();
        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, verifiedScore);
        assertTrue(registry.isVerified(PROVIDER_ID), "score == 800, checker yes");

        // Swap to a checker that always rejects — verification fails again.
        vm.prank(admin);
        registry.setStakeChecker(address(no));
        assertFalse(registry.isVerified(PROVIDER_ID), "checker says no");

        // Drop the score back below threshold.
        vm.prank(admin);
        registry.setStakeChecker(address(yes));
        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, verifiedScore - 1);
        assertFalse(registry.isVerified(PROVIDER_ID), "score < 800 unverifies");
    }

    function test_IsVerified_RequiresRegistered() public {
        TrueStakeChecker yes = new TrueStakeChecker();
        vm.prank(admin);
        registry.setStakeChecker(address(yes));

        assertFalse(registry.isVerified(keccak256("ghost")), "unregistered, unverified");
    }

    function testFuzz_ReputationScoreRoundTrip(uint16 raw) public {
        uint256 score = bound(uint256(raw), 0, registry.MAX_REPUTATION());

        _register(provider, PROVIDER_ID);

        bytes32 oracleRole = registry.ORACLE_ROLE();
        vm.prank(admin);
        registry.grantRole(oracleRole, oracle);

        vm.prank(oracle);
        registry.setReputationScore(PROVIDER_ID, score);

        (, uint256 stored,,,) = registry.providers(PROVIDER_ID);
        assertEq(stored, score, "round trip");
    }
}
