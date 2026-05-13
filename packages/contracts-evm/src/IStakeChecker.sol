// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.27;

interface IStakeChecker {
    function isVerifiedStake(bytes32 providerId) external view returns (bool);
}
