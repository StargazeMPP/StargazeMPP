// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {StargazeEscrow} from "../src/StargazeEscrow.sol";

contract MockUSD is ERC20 {
    constructor() ERC20("PathUSD", "PUSD") {}
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract StargazeEscrowTest is Test {
    StargazeEscrow internal escrow;
    MockUSD internal pathUsd;

    address internal admin = address(0xA11CE);
    address internal router = address(0x4001);
    address internal agent;
    uint256 internal agentKey;
    address internal provider = address(0x7E57);
    address internal feeSink = address(0xFEE);

    function setUp() public {
        (agent, agentKey) = makeAddrAndKey("agent");

        pathUsd = new MockUSD();
        escrow = new StargazeEscrow(address(pathUsd), admin);

        vm.startPrank(admin);
        escrow.grantRole(escrow.ROUTER_ROLE(), router);
        escrow.setRoutingFeeSink(feeSink);
        vm.stopPrank();

        pathUsd.mint(agent, 1_000e6);
        vm.prank(agent);
        pathUsd.approve(address(escrow), type(uint256).max);
    }

    function test_OpenAndSettleSingleVoucher() public {
        bytes32 sessionId = bytes32(uint256(0xCAFE));
        uint256 deposit = 100e6;
        uint256 limit = 80e6;
        uint64 expiresAt = uint64(block.timestamp + 1 days);

        vm.prank(agent);
        escrow.openSession(sessionId, agent, deposit, limit, expiresAt);

        uint256 cumulative = 25e6;
        uint256 nonce = 1;
        uint64 voucherExpiry = uint64(block.timestamp + 1 hours);

        bytes32 structHash = keccak256(
            abi.encode(
                escrow.VOUCHER_TYPEHASH(),
                sessionId,
                agent,
                provider,
                cumulative,
                nonce,
                voucherExpiry
            )
        );
        bytes32 digest = _typedDataDigest(structHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(agentKey, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        StargazeEscrow.VoucherClaim[] memory claims = new StargazeEscrow.VoucherClaim[](1);
        claims[0] = StargazeEscrow.VoucherClaim({
            provider: provider,
            cumulativeAmount: cumulative,
            nonce: nonce,
            expiry: voucherExpiry,
            signature: sig
        });

        vm.prank(router);
        escrow.settle(sessionId, claims);

        uint256 fee = (cumulative * 200) / 10_000;
        assertEq(pathUsd.balanceOf(provider), cumulative);
        assertEq(pathUsd.balanceOf(feeSink), fee);
        // initial balance was 1_000e6; deposit moved into escrow; refund returned
        uint256 expectedAgentBalance = 1_000e6 - deposit + (deposit - cumulative - fee);
        assertEq(pathUsd.balanceOf(agent), expectedAgentBalance);
    }

    function _typedDataDigest(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", escrow.eip712DomainSeparator(), structHash));
    }
}
