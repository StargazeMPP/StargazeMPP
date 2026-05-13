// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

interface IBurnController {
    function burnForReputationVoteFrom(address voter) external;
}

/// @title StargazeRegistry
/// @notice Provider registration, $GAZE stake collection / slashing,
///         reputation score storage, Verified Provider badge issuance.
contract StargazeRegistry is AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    bytes32 public constant ORACLE_ROLE = keccak256("ORACLE_ROLE");
    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");

    /// @notice Minimum $GAZE stake to register as a provider.
    uint256 public constant MIN_STAKE = 50e18;
    /// @notice Stake threshold for Verified Provider tier.
    uint256 public constant VERIFIED_STAKE = 500e18;
    /// @notice Reputation score threshold for Verified Provider tier.
    uint256 public constant VERIFIED_SCORE = 800;
    /// @notice Max reputation score.
    uint256 public constant MAX_REPUTATION = 1000;

    IERC20 public immutable gaze;
    IBurnController public immutable burnController;

    struct Provider {
        address owner;
        uint256 stake;
        uint256 reputationScore;
        bool registered;
        bytes32 categoryHash; // keccak256(category string) — keep on-chain compact
        bytes32 metaCid;      // ipfs / arweave CID of the full provider profile JSON
    }

    mapping(bytes32 providerId => Provider provider) public providers;

    event ProviderRegistered(bytes32 indexed providerId, address indexed owner, uint256 stake, bytes32 categoryHash);
    event ProviderUpdated(bytes32 indexed providerId, bytes32 metaCid);
    event StakeIncreased(bytes32 indexed providerId, uint256 added, uint256 newTotal);
    event ProviderSlashed(bytes32 indexed providerId, uint256 amount, uint256 remaining, string reason);
    event ReputationUpdated(bytes32 indexed providerId, uint256 score);
    event ReputationVoted(bytes32 indexed providerId, address indexed voter, bool accurate);

    error AlreadyRegistered();
    error NotRegistered();
    error StakeTooLow();
    error ScoreOutOfRange();
    error NotProviderOwner();

    constructor(address gazeToken, address burnControllerAddress, address admin) {
        gaze = IERC20(gazeToken);
        burnController = IBurnController(burnControllerAddress);
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    function register(
        bytes32 providerId,
        bytes32 categoryHash,
        bytes32 metaCid,
        uint256 stakeAmount
    ) external nonReentrant {
        if (providers[providerId].registered) revert AlreadyRegistered();
        if (stakeAmount < MIN_STAKE) revert StakeTooLow();
        gaze.safeTransferFrom(msg.sender, address(this), stakeAmount);
        providers[providerId] = Provider({
            owner: msg.sender,
            stake: stakeAmount,
            reputationScore: 500, // neutral midpoint
            registered: true,
            categoryHash: categoryHash,
            metaCid: metaCid
        });
        emit ProviderRegistered(providerId, msg.sender, stakeAmount, categoryHash);
    }

    function updateMeta(bytes32 providerId, bytes32 metaCid) external {
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        if (p.owner != msg.sender) revert NotProviderOwner();
        p.metaCid = metaCid;
        emit ProviderUpdated(providerId, metaCid);
    }

    function increaseStake(bytes32 providerId, uint256 amount) external nonReentrant {
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        gaze.safeTransferFrom(msg.sender, address(this), amount);
        p.stake += amount;
        emit StakeIncreased(providerId, amount, p.stake);
    }

    /// @notice DAO-executed slash. Reduces stake; burned (sent to 0xdead).
    function slash(bytes32 providerId, uint256 amount, string calldata reason) external onlyRole(SLASHER_ROLE) {
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        uint256 actual = amount > p.stake ? p.stake : amount;
        p.stake -= actual;
        gaze.safeTransfer(address(0xdead), actual);
        emit ProviderSlashed(providerId, actual, p.stake, reason);
    }

    /// @notice Oracle posts an updated composite reputation score (0–1000).
    function setReputationScore(bytes32 providerId, uint256 score) external onlyRole(ORACLE_ROLE) {
        if (score > MAX_REPUTATION) revert ScoreOutOfRange();
        Provider storage p = providers[providerId];
        if (!p.registered) revert NotRegistered();
        p.reputationScore = score;
        emit ReputationUpdated(providerId, score);
    }

    /// @notice Agent crowd-vote. Burns 1 $GAZE via BurnController; the score
    ///         math itself is computed off-chain by the Reputation Oracle.
    function castReputationVote(bytes32 providerId, bool accurate) external {
        if (!providers[providerId].registered) revert NotRegistered();
        burnController.burnForReputationVoteFrom(msg.sender);
        emit ReputationVoted(providerId, msg.sender, accurate);
    }

    function isVerified(bytes32 providerId) external view returns (bool) {
        Provider memory p = providers[providerId];
        return p.registered && p.stake >= VERIFIED_STAKE && p.reputationScore >= VERIFIED_SCORE;
    }
}
