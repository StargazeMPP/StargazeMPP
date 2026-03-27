// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/// @title StargazeEscrow
/// @notice Holds PathUSD per MPP session and batch-settles cumulative
///         EIP-712 vouchers against providers on session close (or balance threshold).
/// @dev Voucher hash construction is the only on-chain consumer of the
///      EIP-712 schema mirrored in `packages/shared/src/mpp/voucher.ts`.
contract StargazeEscrow is AccessControl, ReentrancyGuard, EIP712 {
    using SafeERC20 for IERC20;

    bytes32 public constant ROUTER_ROLE = keccak256("ROUTER_ROLE");

    bytes32 public constant VOUCHER_TYPEHASH = keccak256(
        "Voucher(bytes32 sessionId,address agentWallet,address provider,uint256 cumulativeAmount,uint256 nonce,uint64 expiry)"
    );

    /// @notice 2 % routing fee taken from each settled session.
    uint256 public constant ROUTING_FEE_BPS = 200;
    uint256 public constant BPS_DENOMINATOR = 10_000;

    /// @notice The reward / settlement asset on Tempo (PathUSD ERC-20).
    IERC20 public immutable pathUsd;
    /// @notice Where the 2 % routing fee is sent for further $GAZE split.
    address public routingFeeSink;

    struct Session {
        address agentWallet;
        uint256 deposit;
        uint256 spendingLimit;
        bool settled;
        uint64 expiresAt;
    }

    mapping(bytes32 sessionId => Session session) public sessions;
    mapping(bytes32 voucherHash => bool consumed) public consumedVouchers;
    /// @notice Highest cumulative amount seen per (sessionId, provider) — enforces monotonicity on-chain.
    mapping(bytes32 sessionId => mapping(address provider => uint256 lastCumulative)) public lastCumulative;

    event SessionOpened(
        bytes32 indexed sessionId,
        address indexed agentWallet,
        uint256 deposit,
        uint256 spendingLimit,
        uint64 expiresAt
    );
    event VoucherSettled(
        bytes32 indexed sessionId,
        address indexed provider,
        uint256 deltaAmount,
        uint256 cumulativeAmount,
        uint256 nonce
    );
    event SessionSettled(
        bytes32 indexed sessionId,
        uint256 totalToProviders,
        uint256 routingFee,
        uint256 refundToAgent
    );
    event RoutingFeeSinkSet(address indexed previous, address indexed current);

    error AlreadyOpen();
    error UnknownSession();
    error AlreadySettled();
    error SessionExpired();
    error SpendingLimitExceeded();
    error NonMonotonic();
    error BadSignature();
    error VoucherReused();
    error VoucherExpired();
    error RoutingSinkUnset();

    constructor(address pathUsdToken, address admin) EIP712("StargazeMPP", "1") {
        pathUsd = IERC20(pathUsdToken);
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    function setRoutingFeeSink(address sink) external onlyRole(DEFAULT_ADMIN_ROLE) {
        emit RoutingFeeSinkSet(routingFeeSink, sink);
        routingFeeSink = sink;
    }

    /// @notice Agent (or anyone with their approval) opens a session by escrowing PathUSD.
    /// `sessionId` is generated client-side and must be unique.
    function openSession(
        bytes32 sessionId,
        address agentWallet,
        uint256 deposit,
        uint256 spendingLimit,
        uint64 expiresAt
    ) external nonReentrant {
        Session storage s = sessions[sessionId];
        if (s.agentWallet != address(0)) revert AlreadyOpen();
        if (spendingLimit > deposit) revert SpendingLimitExceeded();

        sessions[sessionId] = Session({
            agentWallet: agentWallet,
            deposit: deposit,
            spendingLimit: spendingLimit,
            settled: false,
            expiresAt: expiresAt
        });

        pathUsd.safeTransferFrom(msg.sender, address(this), deposit);
        emit SessionOpened(sessionId, agentWallet, deposit, spendingLimit, expiresAt);
    }

    struct VoucherClaim {
        address provider;
        uint256 cumulativeAmount;
        uint256 nonce;
        uint64 expiry;
        bytes signature;
    }

    /// @notice Batch-settle a session. Only callable by the off-chain payment
    ///         router (which has assembled the voucher batch).
    function settle(bytes32 sessionId, VoucherClaim[] calldata vouchers)
        external
        onlyRole(ROUTER_ROLE)
        nonReentrant
    {
        Session storage s = sessions[sessionId];
        if (s.agentWallet == address(0)) revert UnknownSession();
        if (s.settled) revert AlreadySettled();
        if (routingFeeSink == address(0)) revert RoutingSinkUnset();

        uint256 totalSpend = 0;

        for (uint256 i = 0; i < vouchers.length; i++) {
            VoucherClaim calldata v = vouchers[i];
            if (v.expiry != 0 && block.timestamp > v.expiry) revert VoucherExpired();

            bytes32 structHash = keccak256(
                abi.encode(VOUCHER_TYPEHASH, sessionId, s.agentWallet, v.provider, v.cumulativeAmount, v.nonce, v.expiry)
            );
            bytes32 digest = _hashTypedDataV4(structHash);
            if (consumedVouchers[digest]) revert VoucherReused();

            address signer = ECDSA.recover(digest, v.signature);
            if (signer != s.agentWallet) revert BadSignature();

            uint256 prev = lastCumulative[sessionId][v.provider];
            if (v.cumulativeAmount <= prev) revert NonMonotonic();

            uint256 delta = v.cumulativeAmount - prev;
            lastCumulative[sessionId][v.provider] = v.cumulativeAmount;
            consumedVouchers[digest] = true;

            totalSpend += delta;
            if (totalSpend > s.spendingLimit) revert SpendingLimitExceeded();

            pathUsd.safeTransfer(v.provider, delta);
            emit VoucherSettled(sessionId, v.provider, delta, v.cumulativeAmount, v.nonce);
        }

        uint256 routingFee = (totalSpend * ROUTING_FEE_BPS) / BPS_DENOMINATOR;
        if (routingFee > 0) {
            pathUsd.safeTransfer(routingFeeSink, routingFee);
        }

        uint256 refund = s.deposit - totalSpend - routingFee;
        if (refund > 0) {
            pathUsd.safeTransfer(s.agentWallet, refund);
        }

        s.settled = true;
        emit SessionSettled(sessionId, totalSpend, routingFee, refund);
    }

    function eip712DomainSeparator() external view returns (bytes32) {
        return _domainSeparatorV4();
    }
}
