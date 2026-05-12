//! Shared helpers for the stargaze_anchor litesvm integration tests.

use anchor_lang::{Discriminator, InstructionData};
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use stargaze_anchor::ID as PROGRAM_ID;

// Hard-code the system program id to avoid the deprecated
// `solana_sdk::system_program` module in 2.3 series.
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

pub const PROGRAM_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/deploy/stargaze_anchor.so"
);

/// Create a fresh `LiteSVM`, load the program at its declared id, and return
/// a funded payer keypair. Advances the clock sysvar so `Clock::get()`
/// produces a non-zero `unix_timestamp` inside the program.
pub fn setup_svm() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PROGRAM_ID, PROGRAM_SO)
        .expect("load stargaze_anchor.so");

    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp = 1_700_000_000;
    svm.set_sysvar(&clock);

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("airdrop payer");
    (svm, payer)
}

pub fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], &PROGRAM_ID)
}

pub fn provider_pda(provider_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"provider", provider_id.as_ref()], &PROGRAM_ID)
}

pub fn x402_pda(session_id: &[u8; 32], provider_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"x402", session_id.as_ref(), provider_id.as_ref()],
        &PROGRAM_ID,
    )
}

/// Build the `initialize` instruction.
pub fn ix_initialize(payer: &Pubkey, authority: Pubkey) -> Instruction {
    let (config, _) = config_pda();
    let data = stargaze_anchor::instruction::Initialize { authority }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `register_provider` instruction.
pub fn ix_register_provider(
    owner: &Pubkey,
    provider_id: [u8; 32],
    category_hash: [u8; 32],
    meta_cid: [u8; 32],
) -> Instruction {
    let (config, _) = config_pda();
    let (provider, _) = provider_pda(&provider_id);
    let data = stargaze_anchor::instruction::RegisterProvider {
        provider_id,
        category_hash,
        meta_cid,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(config, false),
            AccountMeta::new(provider, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `cast_reputation_vote` instruction.
pub fn ix_cast_reputation_vote(
    voter: &Pubkey,
    provider_id: [u8; 32],
    accurate: bool,
) -> Instruction {
    let (provider, _) = provider_pda(&provider_id);
    let data = stargaze_anchor::instruction::CastReputationVote {
        provider_id,
        accurate,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*voter, true),
            AccountMeta::new_readonly(provider, false),
        ],
        data,
    }
}

/// Build the `record_x402_receipt` instruction.
pub fn ix_record_x402_receipt(
    payer: &Pubkey,
    session_id: [u8; 32],
    provider_id: [u8; 32],
    amount: u64,
) -> Instruction {
    let (receipt, _) = x402_pda(&session_id, &provider_id);
    let data = stargaze_anchor::instruction::RecordX402Receipt {
        session_id,
        provider_id,
        amount,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(receipt, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Decode the first `Program data:` log into a typed Anchor `Event`,
/// returning `None` if no matching record is found.
pub fn find_event<E: anchor_lang::Event + Discriminator + anchor_lang::AnchorDeserialize>(
    logs: &[String],
) -> Option<E> {
    use base64::Engine;
    for line in logs {
        if let Some(payload) = line.strip_prefix("Program data: ") {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload)
                .ok()?;
            if bytes.len() < E::DISCRIMINATOR.len() {
                continue;
            }
            if &bytes[..E::DISCRIMINATOR.len()] != E::DISCRIMINATOR {
                continue;
            }
            let mut rest = &bytes[E::DISCRIMINATOR.len()..];
            return E::deserialize(&mut rest).ok();
        }
    }
    None
}

