// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ERC20Burnable} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";

/// @title BurnController
/// @notice Atomic burn execution for $GAZE: routing-fee burns (50% of 2%),
///         citation burns (5 GAZE), and reputation-vote burns (1 GAZE).
contract BurnController is AccessControl {
    bytes32 public constant ROUTER_ROLE = keccak256("ROUTER_ROLE");
    bytes32 public constant REGISTRY_ROLE = keccak256("REGISTRY_ROLE");

    /// @notice Citation burn — fired when a Stargaze result is cited in an IP-NFT.
    uint256 public constant CITATION_BURN_AMOUNT = 5e18;
    /// @notice Reputation vote burn — fired per crowd-verification vote.
    uint256 public constant REPUTATION_VOTE_BURN_AMOUNT = 1e18;

    ERC20Burnable public immutable gaze;
    address public stakerPool;

    uint256 public totalBurned;
    uint256 public totalDistributedToStakers;

    event StakerPoolSet(address indexed previous, address indexed current);
    event RoutingFeeProcessed(address indexed payer, uint256 burned, uint256 toStakers);
    event Citation(address indexed citer, uint256 amount);
    event ReputationVote(address indexed voter, uint256 amount);
    event TransferObserved(address indexed from, address indexed to, uint256 amount);

    error ZeroAmount();
    error StakerPoolUnset();

    constructor(address gazeToken, address admin) {
        gaze = ERC20Burnable(gazeToken);
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    function setStakerPool(address pool) external onlyRole(DEFAULT_ADMIN_ROLE) {
        emit StakerPoolSet(stakerPool, pool);
        stakerPool = pool;
    }

    /// @notice Called by the payment router (off-chain settler) after a session settles.
    /// Caller must have approved this contract for `feeAmount` $GAZE.
    function processRoutingFee(uint256 feeAmount) external onlyRole(ROUTER_ROLE) {
        if (feeAmount == 0) revert ZeroAmount();
        if (stakerPool == address(0)) revert StakerPoolUnset();

        uint256 toBurn = feeAmount / 2;
        uint256 toStakers = feeAmount - toBurn;

        if (toBurn > 0) {
            gaze.burnFrom(msg.sender, toBurn);
            totalBurned += toBurn;
        }
        if (toStakers > 0) {
            // Transfer GAZE from the router into the staker pool. Pool is a
            // distributor of rewards (see GAZEToken.notifyRewardAmount).
            require(gaze.transferFrom(msg.sender, stakerPool, toStakers), "BurnController: transfer failed");
            totalDistributedToStakers += toStakers;
        }
        emit RoutingFeeProcessed(msg.sender, toBurn, toStakers);
    }

    function burnForCitation() external {
        gaze.burnFrom(msg.sender, CITATION_BURN_AMOUNT);
        totalBurned += CITATION_BURN_AMOUNT;
        emit Citation(msg.sender, CITATION_BURN_AMOUNT);
    }

    function burnForReputationVote() external {
        gaze.burnFrom(msg.sender, REPUTATION_VOTE_BURN_AMOUNT);
        totalBurned += REPUTATION_VOTE_BURN_AMOUNT;
        emit ReputationVote(msg.sender, REPUTATION_VOTE_BURN_AMOUNT);
    }

    /// @notice Registry-delegated reputation-vote burn. The voter must have
    /// approved this contract for `REPUTATION_VOTE_BURN_AMOUNT` $GAZE; the
    /// registry forwards the call so the voter's allowance is debited rather
    /// than the registry's (which holds none).
    function burnForReputationVoteFrom(address voter) external onlyRole(REGISTRY_ROLE) {
        gaze.burnFrom(voter, REPUTATION_VOTE_BURN_AMOUNT);
        totalBurned += REPUTATION_VOTE_BURN_AMOUNT;
        emit ReputationVote(voter, REPUTATION_VOTE_BURN_AMOUNT);
    }

    /// @notice Hook called by GAZEToken on every non-mint, non-burn transfer.
    /// Currently telemetry-only — kept here so the GAZE token's transfer-hook
    /// ABI is stable across future controller versions.
    function onTransfer(address from, address to, uint256 amount) external {
        emit TransferObserved(from, to, amount);
    }
}
