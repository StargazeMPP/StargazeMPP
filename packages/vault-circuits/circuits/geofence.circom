pragma circom 2.1.6;

include "circomlib/circuits/comparators.circom";

/// Geofence
///
/// Proves a private (lat, lon) point lies within a public axis-aligned
/// bounding box [minLat, maxLat] × [minLon, maxLon] without revealing the
/// point itself. Used by `confidential` tier providers — drone corridors,
/// OFAC attestations, mission boundaries.
///
/// All six signals are unsigned non-negative integers strictly less than
/// 2^N. Callers encode signed micro-degrees by adding a fixed offset
/// (e.g. lat × 1e6 + 2^31) before passing them in, so geographic
/// comparisons survive bn128 field arithmetic.
///
/// Public:  minLat, maxLat, minLon, maxLon
/// Private: lat, lon
///
/// N=32 comfortably covers ±180° in micro-degrees with a 2^31 offset.
template Geofence(N) {
    signal input lat;
    signal input lon;
    signal input minLat;
    signal input maxLat;
    signal input minLon;
    signal input maxLon;

    component latGeMin = LessEqThan(N);
    latGeMin.in[0] <== minLat;
    latGeMin.in[1] <== lat;
    latGeMin.out === 1;

    component latLeMax = LessEqThan(N);
    latLeMax.in[0] <== lat;
    latLeMax.in[1] <== maxLat;
    latLeMax.out === 1;

    component lonGeMin = LessEqThan(N);
    lonGeMin.in[0] <== minLon;
    lonGeMin.in[1] <== lon;
    lonGeMin.out === 1;

    component lonLeMax = LessEqThan(N);
    lonLeMax.in[0] <== lon;
    lonLeMax.in[1] <== maxLon;
    lonLeMax.out === 1;
}

component main { public [minLat, maxLat, minLon, maxLon] } = Geofence(32);
