// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

interface IBurnControllerHook {
    function onTransfer(address from, address to, uint256 amount) external;
}

/// @title GAZEToken
/// @notice Stargaze coordination token. ERC-20 with staking (7-day unstake cooldown)
///         and an optional transfer hook into BurnController for routing telemetry.
/// @dev Reward distribution uses a per-share accumulator pattern; the actual
///      reward asset (PathUSD or USDC) is delivered by the off-chain payment
///      router calling notifyRewardAmount.
contract GAZEToken is ERC20, AccessControl, ReentrancyGuard {
    bytes32 public constant DISTRIBUTOR_ROLE = keccak256("DISTRIBUTOR_ROLE");

    uint256 public constant UNSTAKE_COOLDOWN = 7 days;
    uint256 public constant ACC_PRECISION = 1e18;

    IBurnControllerHook public burnController;

    struct StakeInfo {
        uint256 amount;
        uint256 pendingUnstake;
        uint256 cooldownEndsAt;
        uint256 rewardDebt;
    }

    mapping(address staker => StakeInfo info) public stakeOf;
    uint256 public totalStaked;
    uint256 public accRewardPerShare;

    event BurnControllerSet(address indexed previous, address indexed current);
    event Staked(address indexed staker, uint256 amount);
    event UnstakeRequested(address indexed staker, uint256 amount, uint256 cooldownEndsAt);
    event Unstaked(address indexed staker, uint256 amount);
    event RewardsNotified(uint256 amount, uint256 newAccPerShare);
    event RewardsClaimed(address indexed staker, uint256 amount);

    error ZeroAmount();
    error InsufficientStake();
    error CooldownActive(uint256 endsAt);
    error NothingPending();
    error NoStakers();

    constructor(uint256 initialSupply, address admin) ERC20("Stargaze", "GAZE") {
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _mint(admin, initialSupply);
    }

    function setBurnController(address controller) external onlyRole(DEFAULT_ADMIN_ROLE) {
        address prev = address(burnController);
        burnController = IBurnControllerHook(controller);
        emit BurnControllerSet(prev, controller);
    }

    function stake(uint256 amount) external nonReentrant {
        if (amount == 0) revert ZeroAmount();
        StakeInfo storage s = stakeOf[msg.sender];
        _settleRewards(msg.sender, s);
        _transfer(msg.sender, address(this), amount);
        s.amount += amount;
        totalStaked += amount;
        s.rewardDebt = (s.amount * accRewardPerShare) / ACC_PRECISION;
        emit Staked(msg.sender, amount);
    }

    function requestUnstake(uint256 amount) external nonReentrant {
        StakeInfo storage s = stakeOf[msg.sender];
        if (amount == 0) revert ZeroAmount();
        if (s.amount < amount) revert InsufficientStake();
        _settleRewards(msg.sender, s);
        s.amount -= amount;
        s.pendingUnstake += amount;
        s.cooldownEndsAt = block.timestamp + UNSTAKE_COOLDOWN;
        totalStaked -= amount;
        s.rewardDebt = (s.amount * accRewardPerShare) / ACC_PRECISION;
        emit UnstakeRequested(msg.sender, amount, s.cooldownEndsAt);
    }

    function claimUnstake() external nonReentrant {
        StakeInfo storage s = stakeOf[msg.sender];
        if (block.timestamp < s.cooldownEndsAt) revert CooldownActive(s.cooldownEndsAt);
        uint256 amount = s.pendingUnstake;
        if (amount == 0) revert NothingPending();
        s.pendingUnstake = 0;
        _transfer(address(this), msg.sender, amount);
        emit Unstaked(msg.sender, amount);
    }

    function claimRewards() external nonReentrant returns (uint256 claimed) {
        StakeInfo storage s = stakeOf[msg.sender];
        claimed = _settleRewards(msg.sender, s);
        s.rewardDebt = (s.amount * accRewardPerShare) / ACC_PRECISION;
    }

    /// @notice Called by the payment router after pulling routing-fee rewards.
    /// `amount` is in the reward asset's units (e.g. PathUSD wei). Actual
    /// reward asset transfer/escrow is handled by the router.
    function notifyRewardAmount(uint256 amount) external onlyRole(DISTRIBUTOR_ROLE) {
        if (totalStaked == 0) revert NoStakers();
        accRewardPerShare += (amount * ACC_PRECISION) / totalStaked;
        emit RewardsNotified(amount, accRewardPerShare);
    }

    function pendingRewards(address staker) external view returns (uint256) {
        StakeInfo memory s = stakeOf[staker];
        return (s.amount * accRewardPerShare) / ACC_PRECISION - s.rewardDebt;
    }

    function _settleRewards(address staker, StakeInfo storage s) internal returns (uint256 claimed) {
        uint256 accumulated = (s.amount * accRewardPerShare) / ACC_PRECISION;
        claimed = accumulated - s.rewardDebt;
        if (claimed > 0) emit RewardsClaimed(staker, claimed);
    }

    function _update(address from, address to, uint256 value) internal override {
        super._update(from, to, value);
        if (address(burnController) != address(0) && from != address(0) && to != address(0)) {
            burnController.onTransfer(from, to, value);
        }
    }
}
