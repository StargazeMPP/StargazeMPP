//! Shared helpers for the stargaze_anchor litesvm integration tests.

use anchor_lang::{Discriminator, InstructionData};
use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signer},
};
use stargaze_anchor::ID as PROGRAM_ID;

// Hard-code the system program id to avoid the deprecated
// `solana_sdk::system_program` module in 2.3 series.
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

/// Canonical SPL Token program id.
pub const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!(
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);

/// Canonical Associated Token Account program id.
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!(
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);

/// Canonical Solana incinerator burn address (matches `BURN_DESTINATION` in
/// the on-chain program).
pub const BURN_DESTINATION: Pubkey = solana_sdk::pubkey!(
    "1nc1nerator11111111111111111111111111111111"
);

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

/// Build the `ccip_mirror_score` instruction.
pub fn ix_ccip_mirror_score(
    ccip_router: &Pubkey,
    provider_id: [u8; 32],
    new_score: u16,
) -> Instruction {
    let (config, _) = config_pda();
    let (provider, _) = provider_pda(&provider_id);
    let data = stargaze_anchor::instruction::CcipMirrorScore {
        provider_id,
        new_score,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*ccip_router, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(provider, false),
        ],
        data,
    }
}

/// Build the `dispatch_reputation_to_tempo` instruction.
pub fn ix_dispatch_reputation_to_tempo(
    sender: &Pubkey,
    ccip_router_program: &Pubkey,
    provider_id: [u8; 32],
    dest_chain_selector: u64,
    receiver: Vec<u8>,
    extra_args: Vec<u8>,
) -> Instruction {
    let (config, _) = config_pda();
    let (provider, _) = provider_pda(&provider_id);
    let data = stargaze_anchor::instruction::DispatchReputationToTempo {
        provider_id,
        dest_chain_selector,
        receiver,
        extra_args,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*sender, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(provider, false),
            AccountMeta::new_readonly(*ccip_router_program, false),
        ],
        data,
    }
}

pub fn staking_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"staking_config"], &PROGRAM_ID)
}

pub fn stake_account_pda(provider_id: &[u8; 32], owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"stake", provider_id.as_ref(), owner.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn stake_pool_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"stake_pool_authority"], &PROGRAM_ID)
}

/// Compute the associated token account address for `owner` holding `mint`.
pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(owner, mint)
}

/// Build the `init_staking` instruction.
pub fn ix_init_staking(
    authority: &Pubkey,
    stake_mint: Pubkey,
    min_stake: u64,
    verified_stake: u64,
    cooldown_secs: i64,
) -> Instruction {
    let (config, _) = config_pda();
    let (staking_config, _) = staking_config_pda();
    let data = stargaze_anchor::instruction::InitStaking {
        stake_mint,
        min_stake,
        verified_stake,
        cooldown_secs,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(staking_config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `set_stake_mint` instruction. The pool token account is the
/// associated-token-account address derived from the `stake_pool_authority`
/// PDA + the mint.
pub fn ix_set_stake_mint(authority: &Pubkey, new_mint: Pubkey) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_pool_authority, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&stake_pool_authority, &new_mint);
    let data = stargaze_anchor::instruction::SetStakeMint { new_mint }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(staking_config, false),
            AccountMeta::new_readonly(new_mint, false),
            AccountMeta::new_readonly(stake_pool_authority, false),
            AccountMeta::new(pool_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `stake` instruction.
pub fn ix_stake(
    staker: &Pubkey,
    stake_mint: &Pubkey,
    provider_id: [u8; 32],
    amount: u64,
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_account, _) = stake_account_pda(&provider_id, staker);
    let staker_ata = associated_token_address(staker, stake_mint);
    let (stake_pool_authority, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&stake_pool_authority, stake_mint);
    let data = stargaze_anchor::instruction::Stake {
        provider_id,
        amount,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*staker, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new(stake_account, false),
            AccountMeta::new(*stake_mint, false),
            AccountMeta::new(staker_ata, false),
            AccountMeta::new_readonly(stake_pool_authority, false),
            AccountMeta::new(pool_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `request_unstake` instruction.
pub fn ix_request_unstake(
    staker: &Pubkey,
    provider_id: [u8; 32],
    amount: u64,
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_account, _) = stake_account_pda(&provider_id, staker);
    let data = stargaze_anchor::instruction::RequestUnstake {
        provider_id,
        amount,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*staker, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new(stake_account, false),
        ],
        data,
    }
}

/// Build the `claim_unstake` instruction.
pub fn ix_claim_unstake(
    staker: &Pubkey,
    stake_mint: &Pubkey,
    provider_id: [u8; 32],
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_account, _) = stake_account_pda(&provider_id, staker);
    let staker_ata = associated_token_address(staker, stake_mint);
    let (stake_pool_authority, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&stake_pool_authority, stake_mint);
    let data = stargaze_anchor::instruction::ClaimUnstake { provider_id }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*staker, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new(stake_account, false),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new(staker_ata, false),
            AccountMeta::new_readonly(stake_pool_authority, false),
            AccountMeta::new(pool_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `slash` instruction. Burn destination ATA is the
/// associated-token-account of the canonical incinerator address.
pub fn ix_slash(
    authority: &Pubkey,
    stake_mint: &Pubkey,
    provider_id: [u8; 32],
    staker: Pubkey,
    amount: u64,
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_account, _) = stake_account_pda(&provider_id, &staker);
    let (stake_pool_authority, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&stake_pool_authority, stake_mint);
    let burn_ata = associated_token_address(&BURN_DESTINATION, stake_mint);
    let data = stargaze_anchor::instruction::Slash {
        provider_id,
        staker,
        amount,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new(stake_account, false),
            AccountMeta::new_readonly(*stake_mint, false),
            AccountMeta::new_readonly(stake_pool_authority, false),
            AccountMeta::new(pool_ata, false),
            AccountMeta::new(burn_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Mint helpers — initialise a fresh SPL Token mint and create token accounts
/// directly via the on-chain SPL Token program. These rely on the SPL Token
/// program being available in the LiteSVM instance.
pub fn create_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
) -> Keypair {
    let mint = Keypair::new();
    let rent = svm
        .get_sysvar::<Rent>()
        .minimum_balance(spl_token::state::Mint::LEN);

    let create_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &TOKEN_PROGRAM_ID,
    );
    let init_ix = spl_token::instruction::initialize_mint(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        mint_authority,
        None,
        decimals,
    )
    .expect("initialize_mint ix");

    let blockhash = svm.latest_blockhash();
    let msg = solana_sdk::message::Message::new(&[create_ix, init_ix], Some(&payer.pubkey()));
    let tx = solana_sdk::transaction::Transaction::new(&[payer, &mint], msg, blockhash);
    svm.send_transaction(tx).expect("create mint");

    mint
}

/// Create an associated token account for `owner` holding `mint`. Returns the
/// ATA pubkey.
pub fn create_associated_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = associated_token_address(owner, mint);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    );
    let blockhash = svm.latest_blockhash();
    let msg = solana_sdk::message::Message::new(&[ix], Some(&payer.pubkey()));
    let tx = solana_sdk::transaction::Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx).expect("create ATA");
    ata
}

/// Mint `amount` base units of `mint` to `dest_ata`. Caller signs as the
/// mint authority.
pub fn mint_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    dest_ata: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) {
    let ix = spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        mint,
        dest_ata,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .expect("mint_to ix");
    let blockhash = svm.latest_blockhash();
    let msg = solana_sdk::message::Message::new(&[ix], Some(&payer.pubkey()));
    let tx = solana_sdk::transaction::Transaction::new(&[payer, mint_authority], msg, blockhash);
    svm.send_transaction(tx).expect("mint_to");
}

/// Deserialise a `spl_token::state::Account` from the SVM and return the
/// raw `amount` field (base units).
pub fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let acct = svm.get_account(ata).expect("ATA exists");
    let parsed = spl_token::state::Account::unpack(&acct.data).expect("decode token account");
    parsed.amount
}

/// Insert a minimal lamport-funded system account at `addr`. Useful when the
/// burn destination needs to exist before an ATA is created against it.
pub fn ensure_system_account(svm: &mut LiteSVM, addr: &Pubkey, lamports: u64) {
    if svm.get_account(addr).is_some() {
        return;
    }
    let _ = svm.set_account(
        *addr,
        Account {
            lamports,
            data: vec![],
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
}

/// Advance the SVM clock by `delta` seconds. Used in staking tests to fast
/// forward past the unstake cooldown without polling real wall-time.
pub fn warp_clock(svm: &mut LiteSVM, delta: i64) {
    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp = clock.unix_timestamp.saturating_add(delta);
    svm.set_sysvar(&clock);
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

