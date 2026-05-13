// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

import {IStakeChecker} from "./IStakeChecker.sol";

/// @title StubStakeChecker
/// @notice Temporary stub used until CCIP mirrors Solana stake to Tempo for
///         `isVerified` reads. Always reports providers as having a verified
///         stake; the registry's Verified Provider gate falls back to the
///         reputation-score threshold alone while this stub is wired in.
contract StubStakeChecker is IStakeChecker {
    function isVerifiedStake(bytes32) external pure returns (bool) {
        return true;
    }
}
