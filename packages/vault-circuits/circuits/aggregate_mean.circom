pragma circom 2.1.6;

/// AggregateMean
///
/// Proves the integer mean of N private values without revealing them:
/// asserts `sum(values) == N * claimedMean` over the bn128 field. The
/// publisher is responsible for rounding to integers before submitting —
/// `claimedMean = floor(sum / N)` cannot be enforced inside the circuit
/// because field division is not the same as integer division. Use this
/// variant only when callers tolerate the floor rounding (e.g. cohort
/// HRV averages reported as whole bpm).
///
/// Public input:  claimedMean
/// Private input: values[N]
///
/// N is the cohort size, set at compile time. Pick N to match the smallest
/// cohort your privacy policy allows — smaller leaks more information.
template AggregateMean(N) {
    signal input values[N];
    signal input claimedMean;

    signal partial[N + 1];
    partial[0] <== 0;

    for (var i = 0; i < N; i++) {
        partial[i + 1] <== partial[i] + values[i];
    }

    partial[N] === N * claimedMean;
}

component main { public [claimedMean] } = AggregateMean(8);
