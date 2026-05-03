pragma circom 2.1.6;

/// AggregateSum
///
/// Proves that the sum of N private values equals a publicly claimed sum,
/// without revealing the individual values. Used by ZK-AGGREGATE tier
/// providers (e.g. AxonMed cohort stats — total HRV across an opaque cohort).
///
/// Public input:  claimedSum
/// Private input: values[N]
///
/// N is the cohort size (set at circuit compile time). Smaller circuits
/// are cheaper to prove but leak cohort granularity — pick N to match the
/// minimum cohort size your privacy policy allows.
template AggregateSum(N) {
    signal input values[N];
    signal input claimedSum;

    signal partial[N + 1];
    partial[0] <== 0;

    for (var i = 0; i < N; i++) {
        partial[i + 1] <== partial[i] + values[i];
    }

    partial[N] === claimedSum;
}

component main { public [claimedSum] } = AggregateSum(8);
