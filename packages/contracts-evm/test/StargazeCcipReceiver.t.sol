// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {
    StargazeCcipReceiver,
    IStargazeRegistry,
    Any2EVMMessage,
    EVMTokenAmount
} from "../src/StargazeCcipReceiver.sol";

contract MockRegistry is IStargazeRegistry {
    mapping(bytes32 => uint256) public scores;
    uint256 public callCount;

    function setReputationScore(bytes32 providerId, uint256 score) external {
        scores[providerId] = score;
        callCount += 1;
    }
}

contract StargazeCcipReceiverTest is Test {
    StargazeCcipReceiver internal receiver;
    MockRegistry internal registry;

    address internal admin = address(0xA11CE);
    address internal ccipRouter = address(0xCC1F);

    /// @dev Chainlink's published Solana mainnet selector.
    uint64 internal constant SOLANA_SELECTOR = 5854466;
    bytes internal anchorSender = abi.encode(bytes32("StargazeAnchor111111111111111111"));

    function setUp() public {
        registry = new MockRegistry();
        receiver = new StargazeCcipReceiver(address(registry), admin);

        vm.startPrank(admin);
        receiver.grantRole(receiver.CCIP_ROUTER_ROLE(), ccipRouter);
        receiver.setAllowedSource(SOLANA_SELECTOR, true);
        receiver.setAllowedSender(SOLANA_SELECTOR, anchorSender, true);
        vm.stopPrank();
    }

    function _msg(bytes32 providerId, uint16 score, bytes memory sender, uint64 selector)
        internal
        pure
        returns (Any2EVMMessage memory m)
    {
        m.messageId = keccak256(abi.encode("msg", providerId, score));
        m.sourceChainSelector = selector;
        m.sender = sender;
        m.data = abi.encode(providerId, score);
        m.destTokenAmounts = new EVMTokenAmount[](0);
    }

    function test_HappyPath_WritesScore() public {
        bytes32 providerId = keccak256("mpp32");
        uint16 score = 950;

        Any2EVMMessage memory message = _msg(providerId, score, anchorSender, SOLANA_SELECTOR);
        vm.prank(ccipRouter);
        receiver.ccipReceive(message);

        assertEq(registry.scores(providerId), uint256(score));
        assertEq(registry.callCount(), 1);
    }

    function test_RejectsUnknownSource() public {
        Any2EVMMessage memory message = _msg(keccak256("prov"), 800, anchorSender, 99_999);

        vm.prank(ccipRouter);
        vm.expectRevert(abi.encodeWithSelector(StargazeCcipReceiver.SourceNotAllowed.selector, uint64(99_999)));
        receiver.ccipReceive(message);
    }

    function test_RejectsUnknownSender() public {
        bytes memory impostor = abi.encode(bytes32("imposter11111111111"));
        Any2EVMMessage memory message = _msg(keccak256("prov"), 800, impostor, SOLANA_SELECTOR);

        vm.prank(ccipRouter);
        vm.expectRevert(abi.encodeWithSelector(StargazeCcipReceiver.SenderNotAllowed.selector, SOLANA_SELECTOR));
        receiver.ccipReceive(message);
    }

    function test_RejectsScoreOutOfRange() public {
        Any2EVMMessage memory message = _msg(keccak256("prov"), 1001, anchorSender, SOLANA_SELECTOR);

        vm.prank(ccipRouter);
        vm.expectRevert(abi.encodeWithSelector(StargazeCcipReceiver.InvalidScore.selector, uint16(1001)));
        receiver.ccipReceive(message);
    }

    function test_RejectsNonRouterCaller() public {
        Any2EVMMessage memory message = _msg(keccak256("prov"), 800, anchorSender, SOLANA_SELECTOR);
        vm.prank(address(0xBEEF));
        vm.expectRevert();
        receiver.ccipReceive(message);
    }

    function testFuzz_ScoreRoundTrip(bytes32 providerId, uint16 rawScore) public {
        uint16 score = uint16(bound(uint256(rawScore), 0, 1000));
        Any2EVMMessage memory message = _msg(providerId, score, anchorSender, SOLANA_SELECTOR);
        vm.prank(ccipRouter);
        receiver.ccipReceive(message);
        assertEq(registry.scores(providerId), uint256(score));
    }
}
