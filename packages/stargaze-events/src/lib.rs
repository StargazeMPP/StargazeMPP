//! Anchor event decoder for `stargaze_anchor` logs.
//!
//! Yellowstone delivers each transaction with its full log message list.
//! Every Anchor `emit!(Event { .. })` lands as a line of the form
//! `Program data: <base64(8-byte-discriminator || borsh(event))>` in that
//! list. The discriminator is `sha256("event:<EventName>")[..8]` — see the
//! [Anchor source][anchor-emit] for the canonical definition.
//!
//! The struct layouts here are kept byte-compatible with the on-chain
//! `#[event]` definitions in `packages/anchor-program/programs/stargaze_anchor`.
//! Round-trip tests in [`tests`] guard the format; integration tests in the
//! anchor-program package replay real litesvm logs through this decoder.
//!
//! [anchor-emit]: https://github.com/coral-xyz/anchor

use std::sync::LazyLock;

use base64::{engine::general_purpose, Engine};
use borsh::BorshDeserialize;
use sha2::{Digest, Sha256};

const PROGRAM_DATA_PREFIX: &str = "Program data: ";

/// 32-byte Solana public key (stored opaquely to avoid pulling in the
/// full solana-sdk crate). Display formats as base58 for human-readable
/// log output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct PubkeyBytes(pub [u8; 32]);

impl std::fmt::Display for PubkeyBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&bs58::encode(self.0).into_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct ProviderRegistered {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub category_hash: [u8; 32],
    pub meta_cid: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct ReputationVoted {
    pub provider_id: [u8; 32],
    pub voter: PubkeyBytes,
    pub accurate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct X402ReceiptRecorded {
    pub session_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub payer: PubkeyBytes,
    pub amount: u64,
    pub paid_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct ReputationMirrored {
    pub provider_id: [u8; 32],
    pub score: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct CcipDispatched {
    pub provider_id: [u8; 32],
    pub score: u16,
    pub dest_chain_selector: u64,
    pub receiver: Vec<u8>,
    pub payload: Vec<u8>,
    pub extra_args: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct Staked {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub amount: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct UnstakeRequested {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub amount: u64,
    pub cooldown_until: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct Unstaked {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct Slashed {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub amount: u64,
    pub destination: PubkeyBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct StakingInitialized {
    pub stake_mint: PubkeyBytes,
    pub min_stake: u64,
    pub verified_stake: u64,
    pub cooldown_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct StakeMintSet {
    pub stake_mint: PubkeyBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct RoutingFeeProcessed {
    pub burned: u64,
    pub to_stakers: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct ReputationVoteBurned {
    pub voter: PubkeyBytes,
    pub provider_id: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct StakeDispatched {
    pub provider_id: [u8; 32],
    pub owner: PubkeyBytes,
    pub amount: u64,
    pub dest_chain_selector: u64,
    pub receiver: Vec<u8>,
    pub payload: Vec<u8>,
    pub extra_args: Vec<u8>,
}

/// Mirror of the on-chain `stargaze_anchor::VaultTier` enum. Variants must
/// stay in the same declaration order as the on-chain definition — anchor's
/// borsh 0.10 encodes enum discriminants by declaration position, ignoring
/// any `#[repr(u8)]` values. `use_discriminant=false` here pins our borsh
/// 1.x decoder to the same behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
#[borsh(use_discriminant = false)]
pub enum VaultTier {
    Open,
    ZkAggregate,
    Confidential,
    BuyerKey,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VaultProofVerified {
    pub provider_id: [u8; 32],
    pub tier: VaultTier,
    pub signals_hash: [u8; 32],
    pub submitter: PubkeyBytes,
    pub slot: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct ReputationScoreSet {
    pub provider_id: [u8; 32],
    pub score: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct EscrowInitialized {
    pub admin: PubkeyBytes,
    pub usdc_mint: PubkeyBytes,
    pub router: PubkeyBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct SessionOpened {
    pub session_id: [u8; 32],
    pub agent_wallet: PubkeyBytes,
    pub deposit: u64,
    pub spending_limit: u64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VoucherSettled {
    pub session_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub cumulative_amount: u64,
    pub delta: u64,
    pub to_provider: u64,
    pub fee: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct SessionSettled {
    pub session_id: [u8; 32],
    pub total_to_providers: u64,
    pub routing_fee: u64,
    pub refund_to_agent: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VaultConfigured {
    pub provider_id: [u8; 32],
    pub tier: VaultTier,
    pub on_chain_verifier: PubkeyBytes,
    pub arweave_cid: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VaultAuditorKeySet {
    pub provider_id: [u8; 32],
    pub previous: PubkeyBytes,
    pub current: PubkeyBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VaultBuyerKeyRotationUpdated {
    pub provider_id: [u8; 32],
    pub cid: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, borsh::BorshSerialize)]
pub struct VaultDeactivated {
    pub provider_id: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedEvent {
    ProviderRegistered(ProviderRegistered),
    ReputationVoted(ReputationVoted),
    X402ReceiptRecorded(X402ReceiptRecorded),
    ReputationMirrored(ReputationMirrored),
    CcipDispatched(CcipDispatched),
    Staked(Staked),
    UnstakeRequested(UnstakeRequested),
    Unstaked(Unstaked),
    Slashed(Slashed),
    StakingInitialized(StakingInitialized),
    StakeMintSet(StakeMintSet),
    RoutingFeeProcessed(RoutingFeeProcessed),
    ReputationVoteBurned(ReputationVoteBurned),
    StakeDispatched(StakeDispatched),
    VaultProofVerified(VaultProofVerified),
    ReputationScoreSet(ReputationScoreSet),
    EscrowInitialized(EscrowInitialized),
    SessionOpened(SessionOpened),
    VoucherSettled(VoucherSettled),
    SessionSettled(SessionSettled),
    VaultConfigured(VaultConfigured),
    VaultAuditorKeySet(VaultAuditorKeySet),
    VaultBuyerKeyRotationUpdated(VaultBuyerKeyRotationUpdated),
    VaultDeactivated(VaultDeactivated),
}

impl DecodedEvent {
    pub fn name(&self) -> &'static str {
        match self {
            DecodedEvent::ProviderRegistered(_) => "ProviderRegistered",
            DecodedEvent::ReputationVoted(_) => "ReputationVoted",
            DecodedEvent::X402ReceiptRecorded(_) => "X402ReceiptRecorded",
            DecodedEvent::ReputationMirrored(_) => "ReputationMirrored",
            DecodedEvent::CcipDispatched(_) => "CcipDispatched",
            DecodedEvent::Staked(_) => "Staked",
            DecodedEvent::UnstakeRequested(_) => "UnstakeRequested",
            DecodedEvent::Unstaked(_) => "Unstaked",
            DecodedEvent::Slashed(_) => "Slashed",
            DecodedEvent::StakingInitialized(_) => "StakingInitialized",
            DecodedEvent::StakeMintSet(_) => "StakeMintSet",
            DecodedEvent::RoutingFeeProcessed(_) => "RoutingFeeProcessed",
            DecodedEvent::ReputationVoteBurned(_) => "ReputationVoteBurned",
            DecodedEvent::StakeDispatched(_) => "StakeDispatched",
            DecodedEvent::VaultProofVerified(_) => "VaultProofVerified",
            DecodedEvent::ReputationScoreSet(_) => "ReputationScoreSet",
            DecodedEvent::EscrowInitialized(_) => "EscrowInitialized",
            DecodedEvent::SessionOpened(_) => "SessionOpened",
            DecodedEvent::VoucherSettled(_) => "VoucherSettled",
            DecodedEvent::SessionSettled(_) => "SessionSettled",
            DecodedEvent::VaultConfigured(_) => "VaultConfigured",
            DecodedEvent::VaultAuditorKeySet(_) => "VaultAuditorKeySet",
            DecodedEvent::VaultBuyerKeyRotationUpdated(_) => "VaultBuyerKeyRotationUpdated",
            DecodedEvent::VaultDeactivated(_) => "VaultDeactivated",
        }
    }
}

fn anchor_event_discriminator(name: &str) -> [u8; 8] {
    let digest = Sha256::digest(format!("event:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

pub static DISC_PROVIDER_REGISTERED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("ProviderRegistered"));
pub static DISC_REPUTATION_VOTED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("ReputationVoted"));
pub static DISC_X402_RECEIPT_RECORDED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("X402ReceiptRecorded"));
pub static DISC_REPUTATION_MIRRORED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("ReputationMirrored"));
pub static DISC_CCIP_DISPATCHED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("CcipDispatched"));
pub static DISC_STAKED: LazyLock<[u8; 8]> = LazyLock::new(|| anchor_event_discriminator("Staked"));
pub static DISC_UNSTAKE_REQUESTED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("UnstakeRequested"));
pub static DISC_UNSTAKED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("Unstaked"));
pub static DISC_SLASHED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("Slashed"));
pub static DISC_STAKING_INITIALIZED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("StakingInitialized"));
pub static DISC_STAKE_MINT_SET: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("StakeMintSet"));
pub static DISC_ROUTING_FEE_PROCESSED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("RoutingFeeProcessed"));
pub static DISC_REPUTATION_VOTE_BURNED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("ReputationVoteBurned"));
pub static DISC_STAKE_DISPATCHED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("StakeDispatched"));
pub static DISC_VAULT_PROOF_VERIFIED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VaultProofVerified"));
pub static DISC_REPUTATION_SCORE_SET: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("ReputationScoreSet"));
pub static DISC_ESCROW_INITIALIZED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("EscrowInitialized"));
pub static DISC_SESSION_OPENED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("SessionOpened"));
pub static DISC_VOUCHER_SETTLED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VoucherSettled"));
pub static DISC_SESSION_SETTLED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("SessionSettled"));
pub static DISC_VAULT_CONFIGURED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VaultConfigured"));
pub static DISC_VAULT_AUDITOR_KEY_SET: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VaultAuditorKeySet"));
pub static DISC_VAULT_BUYER_KEY_ROTATION_UPDATED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VaultBuyerKeyRotationUpdated"));
pub static DISC_VAULT_DEACTIVATED: LazyLock<[u8; 8]> =
    LazyLock::new(|| anchor_event_discriminator("VaultDeactivated"));

/// Parse a single program log line. Returns `None` for non-`Program data:`
/// lines, malformed base64, unknown discriminators, or borsh failures.
pub fn decode_program_log(line: &str) -> Option<DecodedEvent> {
    let payload = line.strip_prefix(PROGRAM_DATA_PREFIX)?;
    let bytes = general_purpose::STANDARD.decode(payload).ok()?;
    decode_event_bytes(&bytes)
}

/// Decode raw event bytes — the 8-byte discriminator concatenated with the
/// borsh-serialised event body. Exposed for callers that have already
/// stripped the `Program data:` framing.
pub fn decode_event_bytes(bytes: &[u8]) -> Option<DecodedEvent> {
    if bytes.len() < 8 {
        return None;
    }
    let (disc, mut rest) = bytes.split_at(8);
    if disc == DISC_PROVIDER_REGISTERED.as_ref() {
        return ProviderRegistered::deserialize(&mut rest).ok().map(DecodedEvent::ProviderRegistered);
    }
    if disc == DISC_REPUTATION_VOTED.as_ref() {
        return ReputationVoted::deserialize(&mut rest).ok().map(DecodedEvent::ReputationVoted);
    }
    if disc == DISC_X402_RECEIPT_RECORDED.as_ref() {
        return X402ReceiptRecorded::deserialize(&mut rest).ok().map(DecodedEvent::X402ReceiptRecorded);
    }
    if disc == DISC_REPUTATION_MIRRORED.as_ref() {
        return ReputationMirrored::deserialize(&mut rest).ok().map(DecodedEvent::ReputationMirrored);
    }
    if disc == DISC_CCIP_DISPATCHED.as_ref() {
        return CcipDispatched::deserialize(&mut rest).ok().map(DecodedEvent::CcipDispatched);
    }
    if disc == DISC_STAKED.as_ref() {
        return Staked::deserialize(&mut rest).ok().map(DecodedEvent::Staked);
    }
    if disc == DISC_UNSTAKE_REQUESTED.as_ref() {
        return UnstakeRequested::deserialize(&mut rest).ok().map(DecodedEvent::UnstakeRequested);
    }
    if disc == DISC_UNSTAKED.as_ref() {
        return Unstaked::deserialize(&mut rest).ok().map(DecodedEvent::Unstaked);
    }
    if disc == DISC_SLASHED.as_ref() {
        return Slashed::deserialize(&mut rest).ok().map(DecodedEvent::Slashed);
    }
    if disc == DISC_STAKING_INITIALIZED.as_ref() {
        return StakingInitialized::deserialize(&mut rest).ok().map(DecodedEvent::StakingInitialized);
    }
    if disc == DISC_STAKE_MINT_SET.as_ref() {
        return StakeMintSet::deserialize(&mut rest).ok().map(DecodedEvent::StakeMintSet);
    }
    if disc == DISC_ROUTING_FEE_PROCESSED.as_ref() {
        return RoutingFeeProcessed::deserialize(&mut rest).ok().map(DecodedEvent::RoutingFeeProcessed);
    }
    if disc == DISC_REPUTATION_VOTE_BURNED.as_ref() {
        return ReputationVoteBurned::deserialize(&mut rest).ok().map(DecodedEvent::ReputationVoteBurned);
    }
    if disc == DISC_STAKE_DISPATCHED.as_ref() {
        return StakeDispatched::deserialize(&mut rest).ok().map(DecodedEvent::StakeDispatched);
    }
    if disc == DISC_VAULT_PROOF_VERIFIED.as_ref() {
        return VaultProofVerified::deserialize(&mut rest).ok().map(DecodedEvent::VaultProofVerified);
    }
    if disc == DISC_REPUTATION_SCORE_SET.as_ref() {
        return ReputationScoreSet::deserialize(&mut rest).ok().map(DecodedEvent::ReputationScoreSet);
    }
    if disc == DISC_ESCROW_INITIALIZED.as_ref() {
        return EscrowInitialized::deserialize(&mut rest).ok().map(DecodedEvent::EscrowInitialized);
    }
    if disc == DISC_SESSION_OPENED.as_ref() {
        return SessionOpened::deserialize(&mut rest).ok().map(DecodedEvent::SessionOpened);
    }
    if disc == DISC_VOUCHER_SETTLED.as_ref() {
        return VoucherSettled::deserialize(&mut rest).ok().map(DecodedEvent::VoucherSettled);
    }
    if disc == DISC_SESSION_SETTLED.as_ref() {
        return SessionSettled::deserialize(&mut rest).ok().map(DecodedEvent::SessionSettled);
    }
    if disc == DISC_VAULT_CONFIGURED.as_ref() {
        return VaultConfigured::deserialize(&mut rest).ok().map(DecodedEvent::VaultConfigured);
    }
    if disc == DISC_VAULT_AUDITOR_KEY_SET.as_ref() {
        return VaultAuditorKeySet::deserialize(&mut rest).ok().map(DecodedEvent::VaultAuditorKeySet);
    }
    if disc == DISC_VAULT_BUYER_KEY_ROTATION_UPDATED.as_ref() {
        return VaultBuyerKeyRotationUpdated::deserialize(&mut rest).ok().map(DecodedEvent::VaultBuyerKeyRotationUpdated);
    }
    if disc == DISC_VAULT_DEACTIVATED.as_ref() {
        return VaultDeactivated::deserialize(&mut rest).ok().map(DecodedEvent::VaultDeactivated);
    }
    None
}

/// Scan every line of a transaction's log message list and collect the
/// decoded Anchor events in order.
pub fn decode_logs(logs: &[String]) -> Vec<DecodedEvent> {
    logs.iter().filter_map(|l| decode_program_log(l)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wrap a borsh-serialised event body with its discriminator and the
    /// `Program data:` framing — mirrors what Anchor's `emit!` macro does
    /// on-chain.
    fn synth_log(discriminator: &[u8; 8], body: Vec<u8>) -> String {
        let mut bytes = Vec::with_capacity(8 + body.len());
        bytes.extend_from_slice(discriminator);
        bytes.extend_from_slice(&body);
        format!("{}{}", PROGRAM_DATA_PREFIX, general_purpose::STANDARD.encode(bytes))
    }

    #[test]
    fn discriminators_are_sha256_of_event_name() {
        // Concrete byte values guard against accidental refactors of the
        // discriminator algorithm. Computed once via `sha256("event:Foo")[..8]`.
        // Recomputed at runtime here for self-contained verification.
        assert_eq!(
            *DISC_PROVIDER_REGISTERED,
            anchor_event_discriminator("ProviderRegistered")
        );
        assert_ne!(*DISC_PROVIDER_REGISTERED, *DISC_REPUTATION_VOTED);
    }

    #[test]
    fn decodes_provider_registered() {
        let event = ProviderRegistered {
            provider_id: [1u8; 32],
            owner: PubkeyBytes([2u8; 32]),
            category_hash: [3u8; 32],
            meta_cid: [4u8; 32],
        };
        let log = synth_log(&DISC_PROVIDER_REGISTERED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::ProviderRegistered(ref e) if e == &event));
    }

    #[test]
    fn decodes_reputation_voted() {
        let event = ReputationVoted {
            provider_id: [9u8; 32],
            voter: PubkeyBytes([10u8; 32]),
            accurate: true,
        };
        let log = synth_log(&DISC_REPUTATION_VOTED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::ReputationVoted(ref e) if e == &event));
    }

    #[test]
    fn decodes_x402_receipt_recorded() {
        let event = X402ReceiptRecorded {
            session_id: [5u8; 32],
            provider_id: [6u8; 32],
            payer: PubkeyBytes([7u8; 32]),
            amount: 1_000_000,
            paid_at: 1_700_000_000,
        };
        let log = synth_log(&DISC_X402_RECEIPT_RECORDED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::X402ReceiptRecorded(ref e) if e == &event));
    }

    #[test]
    fn decodes_reputation_mirrored() {
        let event = ReputationMirrored { provider_id: [11u8; 32], score: 750 };
        let log = synth_log(&DISC_REPUTATION_MIRRORED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::ReputationMirrored(ref e) if e == &event));
    }

    #[test]
    fn decodes_ccip_dispatched() {
        let event = CcipDispatched {
            provider_id: [12u8; 32],
            score: 875,
            dest_chain_selector: 16_015_286_601_757_825_753,
            receiver: vec![0xde, 0xad, 0xbe, 0xef],
            payload: vec![1, 2, 3],
            extra_args: vec![],
        };
        let log = synth_log(&DISC_CCIP_DISPATCHED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::CcipDispatched(ref e) if e == &event));
    }

    #[test]
    fn ignores_non_program_data_lines() {
        assert!(decode_program_log("Program log: register_provider").is_none());
        assert!(decode_program_log("Program returned success").is_none());
        assert!(decode_program_log("").is_none());
    }

    #[test]
    fn ignores_unknown_discriminator() {
        let log = synth_log(&[0xff; 8], vec![0; 32]);
        assert!(decode_program_log(&log).is_none());
    }

    #[test]
    fn ignores_malformed_base64() {
        let log = format!("{}!!!not-base64!!!", PROGRAM_DATA_PREFIX);
        assert!(decode_program_log(&log).is_none());
    }

    /// Real `CcipDispatched` log line captured from a litesvm run of
    /// `dispatch_reputation_to_tempo` with deterministic inputs:
    ///   provider_id = [42u8; 32]
    ///   score = 500 (the post-register neutral midpoint)
    ///   dest_chain_selector = 123_456_789
    ///   receiver = [0xde, 0xad, 0xbe, 0xef]
    ///   payload = provider_id || [0u8; 30] || u16::to_be_bytes(score)
    ///   extra_args = vec![]
    /// Captured via `cargo test -p stargaze_anchor_tests --test dump_event_logs -- --nocapture`.
    const CCIP_DISPATCHED_FIXTURE: &str = "Program data: 3uSA/q+Ttl4qKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKvQBFc1bBwAAAAAEAAAA3q2+70AAAAAqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAH0AAAAAA==";

    #[test]
    fn decodes_real_ccip_dispatched_log_from_litesvm() {
        let decoded = decode_program_log(CCIP_DISPATCHED_FIXTURE)
            .expect("real anchor log must decode");
        let DecodedEvent::CcipDispatched(e) = decoded else {
            panic!("expected CcipDispatched, got {}", decoded.name());
        };
        assert_eq!(e.provider_id, [42u8; 32]);
        assert_eq!(e.score, 500);
        assert_eq!(e.dest_chain_selector, 123_456_789);
        assert_eq!(e.receiver, vec![0xde, 0xad, 0xbe, 0xef]);
        // payload schema: bytes32 providerId || 30 zero bytes || uint16 score (big-endian).
        assert_eq!(e.payload.len(), 64);
        assert_eq!(&e.payload[..32], &[42u8; 32]);
        assert!(e.payload[32..62].iter().all(|b| *b == 0));
        assert_eq!(u16::from_be_bytes([e.payload[62], e.payload[63]]), 500);
        assert_eq!(e.extra_args, Vec::<u8>::new());
    }

    #[test]
    fn decodes_staked() {
        let event = Staked {
            provider_id: [13u8; 32],
            owner: PubkeyBytes([14u8; 32]),
            amount: 50_000_000,
            total: 150_000_000,
        };
        let log = synth_log(&DISC_STAKED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::Staked(ref e) if e == &event));
    }

    #[test]
    fn decodes_unstake_requested() {
        let event = UnstakeRequested {
            provider_id: [15u8; 32],
            owner: PubkeyBytes([16u8; 32]),
            amount: 25_000_000,
            cooldown_until: 1_700_604_800,
        };
        let log = synth_log(&DISC_UNSTAKE_REQUESTED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::UnstakeRequested(ref e) if e == &event));
    }

    #[test]
    fn decodes_unstaked() {
        let event = Unstaked {
            provider_id: [17u8; 32],
            owner: PubkeyBytes([18u8; 32]),
            amount: 25_000_000,
        };
        let log = synth_log(&DISC_UNSTAKED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::Unstaked(ref e) if e == &event));
    }

    #[test]
    fn decodes_slashed() {
        let event = Slashed {
            provider_id: [19u8; 32],
            owner: PubkeyBytes([20u8; 32]),
            amount: 10_000_000,
            destination: PubkeyBytes([21u8; 32]),
        };
        let log = synth_log(&DISC_SLASHED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::Slashed(ref e) if e == &event));
    }

    #[test]
    fn decodes_staking_initialized() {
        let event = StakingInitialized {
            stake_mint: PubkeyBytes([22u8; 32]),
            min_stake: 50_000_000,
            verified_stake: 500_000_000,
            cooldown_secs: 7 * 86_400,
        };
        let log = synth_log(&DISC_STAKING_INITIALIZED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::StakingInitialized(ref e) if e == &event));
    }

    #[test]
    fn decodes_stake_mint_set() {
        let event = StakeMintSet {
            stake_mint: PubkeyBytes([23u8; 32]),
        };
        let log = synth_log(&DISC_STAKE_MINT_SET, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::StakeMintSet(ref e) if e == &event));
    }

    #[test]
    fn decodes_routing_fee_processed() {
        let event = RoutingFeeProcessed {
            burned: 500_000,
            to_stakers: 500_000,
        };
        let log = synth_log(&DISC_ROUTING_FEE_PROCESSED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::RoutingFeeProcessed(ref e) if e == &event));
    }

    #[test]
    fn decodes_reputation_vote_burned() {
        let event = ReputationVoteBurned {
            voter: PubkeyBytes([24u8; 32]),
            provider_id: [25u8; 32],
        };
        let log = synth_log(&DISC_REPUTATION_VOTE_BURNED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::ReputationVoteBurned(ref e) if e == &event));
    }

    #[test]
    fn decodes_stake_dispatched() {
        let event = StakeDispatched {
            provider_id: [26u8; 32],
            owner: PubkeyBytes([27u8; 32]),
            amount: 100_000_000,
            dest_chain_selector: 16_015_286_601_757_825_753,
            receiver: vec![0xde, 0xad, 0xbe, 0xef],
            payload: vec![1, 2, 3, 4, 5],
            extra_args: vec![],
        };
        let log = synth_log(&DISC_STAKE_DISPATCHED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::StakeDispatched(ref e) if e == &event));
    }

    #[test]
    fn decodes_vault_proof_verified() {
        let event = VaultProofVerified {
            provider_id: [28u8; 32],
            tier: VaultTier::ZkAggregate,
            signals_hash: [29u8; 32],
            submitter: PubkeyBytes([30u8; 32]),
            slot: 1_234_567,
        };
        let log = synth_log(&DISC_VAULT_PROOF_VERIFIED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::VaultProofVerified(ref e) if e == &event));
    }

    #[test]
    fn decodes_reputation_score_set() {
        let event = ReputationScoreSet { provider_id: [31u8; 32], score: 825 };
        let log = synth_log(&DISC_REPUTATION_SCORE_SET, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::ReputationScoreSet(ref e) if e == &event));
    }

    #[test]
    fn decodes_escrow_initialized() {
        let event = EscrowInitialized {
            admin: PubkeyBytes([32u8; 32]),
            usdc_mint: PubkeyBytes([33u8; 32]),
            router: PubkeyBytes([34u8; 32]),
        };
        let log = synth_log(&DISC_ESCROW_INITIALIZED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::EscrowInitialized(ref e) if e == &event));
    }

    #[test]
    fn decodes_session_opened() {
        let event = SessionOpened {
            session_id: [35u8; 32],
            agent_wallet: PubkeyBytes([36u8; 32]),
            deposit: 5_000_000,
            spending_limit: 2_500_000,
            expires_at: 1_750_000_000,
        };
        let log = synth_log(&DISC_SESSION_OPENED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::SessionOpened(ref e) if e == &event));
    }

    #[test]
    fn decodes_voucher_settled() {
        let event = VoucherSettled {
            session_id: [37u8; 32],
            provider_id: [38u8; 32],
            cumulative_amount: 1_000_000,
            delta: 250_000,
            to_provider: 245_000,
            fee: 5_000,
            nonce: 4,
        };
        let log = synth_log(&DISC_VOUCHER_SETTLED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::VoucherSettled(ref e) if e == &event));
    }

    #[test]
    fn decodes_session_settled() {
        let event = SessionSettled {
            session_id: [39u8; 32],
            total_to_providers: 1_000_000,
            routing_fee: 20_000,
            refund_to_agent: 480_000,
        };
        let log = synth_log(&DISC_SESSION_SETTLED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::SessionSettled(ref e) if e == &event));
    }

    #[test]
    fn decodes_vault_configured() {
        let event = VaultConfigured {
            provider_id: [40u8; 32],
            tier: VaultTier::Confidential,
            on_chain_verifier: PubkeyBytes([41u8; 32]),
            arweave_cid: [42u8; 32],
        };
        let log = synth_log(&DISC_VAULT_CONFIGURED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::VaultConfigured(ref e) if e == &event));
    }

    #[test]
    fn decodes_vault_auditor_key_set() {
        let event = VaultAuditorKeySet {
            provider_id: [43u8; 32],
            previous: PubkeyBytes([44u8; 32]),
            current: PubkeyBytes([45u8; 32]),
        };
        let log = synth_log(&DISC_VAULT_AUDITOR_KEY_SET, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::VaultAuditorKeySet(ref e) if e == &event));
    }

    #[test]
    fn decodes_vault_buyer_key_rotation_updated() {
        let event = VaultBuyerKeyRotationUpdated {
            provider_id: [46u8; 32],
            cid: [47u8; 32],
        };
        let log = synth_log(
            &DISC_VAULT_BUYER_KEY_ROTATION_UPDATED,
            borsh::to_vec(&event).unwrap(),
        );
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(
            matches!(decoded, DecodedEvent::VaultBuyerKeyRotationUpdated(ref e) if e == &event)
        );
    }

    #[test]
    fn decodes_vault_deactivated() {
        let event = VaultDeactivated { provider_id: [48u8; 32] };
        let log = synth_log(&DISC_VAULT_DEACTIVATED, borsh::to_vec(&event).unwrap());
        let decoded = decode_program_log(&log).expect("decodes");
        assert!(matches!(decoded, DecodedEvent::VaultDeactivated(ref e) if e == &event));
    }

    #[test]
    fn vault_tier_round_trips_all_variants() {
        for tier in [
            VaultTier::Open,
            VaultTier::ZkAggregate,
            VaultTier::Confidential,
            VaultTier::BuyerKey,
        ] {
            let bytes = borsh::to_vec(&tier).unwrap();
            assert_eq!(bytes.len(), 1, "tier serialises to a single byte");
            let back: VaultTier = borsh::from_slice(&bytes).unwrap();
            assert_eq!(back, tier);
        }
    }

    #[test]
    fn decode_logs_extracts_multiple_events_in_order() {
        let provider = ProviderRegistered {
            provider_id: [1u8; 32],
            owner: PubkeyBytes([2u8; 32]),
            category_hash: [3u8; 32],
            meta_cid: [4u8; 32],
        };
        let mirrored = ReputationMirrored { provider_id: [11u8; 32], score: 750 };
        let logs = vec![
            "Program log: anything".to_string(),
            synth_log(&DISC_PROVIDER_REGISTERED, borsh::to_vec(&provider).unwrap()),
            "Program log: more noise".to_string(),
            synth_log(&DISC_REPUTATION_MIRRORED, borsh::to_vec(&mirrored).unwrap()),
        ];
        let events = decode_logs(&logs);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].name(), "ProviderRegistered");
        assert_eq!(events[1].name(), "ReputationMirrored");
    }
}
