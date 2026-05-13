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

/// Build the `dispatch_stake_to_tempo` instruction.
pub fn ix_dispatch_stake_to_tempo(
    sender: &Pubkey,
    ccip_router_program: &Pubkey,
    provider_id: [u8; 32],
    owner: Pubkey,
    dest_chain_selector: u64,
    receiver: Vec<u8>,
    extra_args: Vec<u8>,
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let (stake_account, _) = stake_account_pda(&provider_id, &owner);
    let data = stargaze_anchor::instruction::DispatchStakeToTempo {
        provider_id,
        owner,
        dest_chain_selector,
        receiver,
        extra_args,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*sender, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new_readonly(stake_account, false),
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

pub fn staker_reward_pool_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"staker_reward_pool"], &PROGRAM_ID)
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

/// Build the `process_routing_fee_burn` instruction. The reward-pool ATA is
/// derived from the `staker_reward_pool_authority` PDA + the stake mint and
/// is created on first call via `init_if_needed`.
pub fn ix_process_routing_fee_burn(
    authority: &Pubkey,
    stake_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let authority_ata = associated_token_address(authority, stake_mint);
    let (reward_pool_auth, _) = staker_reward_pool_authority_pda();
    let reward_pool_ata = associated_token_address(&reward_pool_auth, stake_mint);
    let data = stargaze_anchor::instruction::ProcessRoutingFeeBurn { amount }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(staking_config, false),
            AccountMeta::new(*stake_mint, false),
            AccountMeta::new(authority_ata, false),
            AccountMeta::new_readonly(reward_pool_auth, false),
            AccountMeta::new(reward_pool_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `reputation_vote_burn` instruction.
pub fn ix_reputation_vote_burn(
    voter: &Pubkey,
    stake_mint: &Pubkey,
    provider_id: [u8; 32],
) -> Instruction {
    let (staking_config, _) = staking_config_pda();
    let voter_ata = associated_token_address(voter, stake_mint);
    let data = stargaze_anchor::instruction::ReputationVoteBurn { provider_id }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*voter, true),
            AccountMeta::new_readonly(staking_config, false),
            AccountMeta::new(*stake_mint, false),
            AccountMeta::new(voter_ata, false),
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

/// Deserialise a `spl_token::state::Mint` from the SVM and return the current
/// total supply (base units). Used by the burn-ladder tests to verify that
/// `token::burn` truly reduces SPL supply.
pub fn mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let acct = svm.get_account(mint).expect("mint exists");
    let parsed = spl_token::state::Mint::unpack(&acct.data).expect("decode mint");
    parsed.supply
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

// ============ ESCROW: PDA helpers ============

/// Canonical Ed25519 native program id (`Ed25519SigVerify111111111111111111111111111`).
pub const ED25519_PROGRAM_ID: Pubkey = solana_sdk::pubkey!(
    "Ed25519SigVerify111111111111111111111111111"
);

/// Canonical instructions sysvar id (`Sysvar1nstructions1111111111111111111111111`).
pub const INSTRUCTIONS_SYSVAR_ID: Pubkey = solana_sdk::pubkey!(
    "Sysvar1nstructions1111111111111111111111111"
);

pub fn usdc_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"usdc_config"], &PROGRAM_ID)
}

pub fn session_pda(session_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"session", session_id.as_ref()], &PROGRAM_ID)
}

pub fn session_vault_authority_pda(session_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"session_vault", session_id.as_ref()], &PROGRAM_ID)
}

pub fn routing_fee_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"routing_fee_vault"], &PROGRAM_ID)
}

pub fn voucher_cursor_pda(session_id: &[u8; 32], provider_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"voucher_cursor", session_id.as_ref(), provider_id.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn consumed_voucher_pda(session_id: &[u8; 32], message_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"voucher", session_id.as_ref(), message_hash.as_ref()],
        &PROGRAM_ID,
    )
}

// ============ ESCROW: voucher message helpers ============

/// Domain separator. Must stay byte-identical to `VOUCHER_DOMAIN_TAG` on-chain.
pub const VOUCHER_DOMAIN_TAG: [u8; 21] = *b"StargazeMPP/Voucher/1";

/// Build the canonical 133-byte voucher message exactly as the on-chain
/// program expects. Layout: `domain_tag (21) || session_id (32) ||
/// agent_wallet (32) || provider_id (32) || cumulative_amount_le (8) ||
/// nonce_le (8)`.
pub fn build_voucher_message(
    session_id: &[u8; 32],
    agent_wallet: &Pubkey,
    provider_id: &[u8; 32],
    cumulative_amount: u64,
    nonce: u64,
) -> [u8; 133] {
    let mut buf = [0u8; 133];
    buf[..21].copy_from_slice(&VOUCHER_DOMAIN_TAG);
    buf[21..53].copy_from_slice(session_id);
    buf[53..85].copy_from_slice(&agent_wallet.to_bytes());
    buf[85..117].copy_from_slice(provider_id);
    buf[117..125].copy_from_slice(&cumulative_amount.to_le_bytes());
    buf[125..133].copy_from_slice(&nonce.to_le_bytes());
    buf
}

/// SHA-256 of the voucher message bytes — used as the seed component for
/// the `consumed_voucher` PDA.
pub fn voucher_message_hash(message: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(message);
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

/// Sign the voucher message with the agent's ed25519 secret key. Returns the
/// 64-byte signature in the standard Ed25519 representation.
pub fn sign_voucher(agent: &Keypair, message: &[u8]) -> [u8; 64] {
    // The 64-byte secret is the concatenation of the seed (32B) and the public
    // key (32B) in solana_sdk::Keypair; ed25519-dalek 1.0's Keypair::from_bytes
    // accepts exactly that layout.
    let dalek_kp = ed25519_dalek::Keypair::from_bytes(&agent.to_bytes())
        .expect("agent keypair -> dalek");
    let sig = ed25519_dalek::Signer::sign(&dalek_kp, message);
    sig.to_bytes()
}

/// Build an Ed25519 precompile instruction that verifies `signature` of
/// `message` by `pubkey`. Layout: 2-byte header + 14-byte offsets + 32B
/// pubkey + 64B signature + N-byte message; matches the on-chain parser.
pub fn build_ed25519_ix(pubkey: &Pubkey, signature: &[u8; 64], message: &[u8]) -> Instruction {
    let pubkey_bytes = pubkey.to_bytes();
    solana_ed25519_program::new_ed25519_instruction_with_signature(
        message,
        signature,
        &pubkey_bytes,
    )
}

// ============ ESCROW: instruction builders ============

/// Build the `init_escrow` instruction.
pub fn ix_init_escrow(authority: &Pubkey, usdc_mint: Pubkey, router: Pubkey) -> Instruction {
    let (config, _) = config_pda();
    let (usdc_config, _) = usdc_config_pda();
    let data = stargaze_anchor::instruction::InitEscrow { usdc_mint, router }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(usdc_config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `open_session` instruction. Caller pays rent + USDC.
pub fn ix_open_session(
    agent: &Pubkey,
    usdc_mint: &Pubkey,
    session_id: [u8; 32],
    deposit: u64,
    spending_limit: u64,
    expires_at: i64,
) -> Instruction {
    let (usdc_config, _) = usdc_config_pda();
    let (session, _) = session_pda(&session_id);
    let (vault_authority, _) = session_vault_authority_pda(&session_id);
    let session_vault_ata = associated_token_address(&vault_authority, usdc_mint);
    let agent_ata = associated_token_address(agent, usdc_mint);
    let data = stargaze_anchor::instruction::OpenSession {
        session_id: Pubkey::new_from_array(session_id),
        deposit,
        spending_limit,
        expires_at,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*agent, true),
            AccountMeta::new_readonly(usdc_config, false),
            AccountMeta::new(session, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(session_vault_ata, false),
            AccountMeta::new(agent_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the program `settle` instruction. The caller MUST prepend an
/// `Ed25519` precompile ix carrying the matching signed message — see
/// `build_ed25519_ix` and `build_voucher_message`.
#[allow(clippy::too_many_arguments)]
pub fn ix_settle(
    router: &Pubkey,
    session_id: [u8; 32],
    provider_id: [u8; 32],
    provider_owner: &Pubkey,
    usdc_mint: &Pubkey,
    cumulative_amount: u64,
    nonce: u64,
    message_hash: [u8; 32],
) -> Instruction {
    let (usdc_config, _) = usdc_config_pda();
    let (session, _) = session_pda(&session_id);
    let (vault_authority, _) = session_vault_authority_pda(&session_id);
    let session_vault_ata = associated_token_address(&vault_authority, usdc_mint);
    let provider_ata = associated_token_address(provider_owner, usdc_mint);
    let (fee_vault_authority, _) = routing_fee_vault_authority_pda();
    let routing_fee_vault_ata = associated_token_address(&fee_vault_authority, usdc_mint);
    let (cursor, _) = voucher_cursor_pda(&session_id, &provider_id);
    let (consumed_voucher, _) = consumed_voucher_pda(&session_id, &message_hash);
    let data = stargaze_anchor::instruction::Settle {
        session_id: Pubkey::new_from_array(session_id),
        provider_id: Pubkey::new_from_array(provider_id),
        cumulative_amount,
        nonce,
        message_hash: Pubkey::new_from_array(message_hash),
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*router, true),
            AccountMeta::new_readonly(usdc_config, false),
            AccountMeta::new(session, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(session_vault_ata, false),
            AccountMeta::new(provider_ata, false),
            AccountMeta::new_readonly(fee_vault_authority, false),
            AccountMeta::new_readonly(*usdc_mint, false),
            AccountMeta::new(routing_fee_vault_ata, false),
            AccountMeta::new(cursor, false),
            AccountMeta::new(consumed_voucher, false),
            AccountMeta::new_readonly(INSTRUCTIONS_SYSVAR_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `close_session` instruction.
pub fn ix_close_session(
    caller: &Pubkey,
    session_id: [u8; 32],
    agent_wallet: &Pubkey,
    usdc_mint: &Pubkey,
) -> Instruction {
    let (usdc_config, _) = usdc_config_pda();
    let (session, _) = session_pda(&session_id);
    let (vault_authority, _) = session_vault_authority_pda(&session_id);
    let session_vault_ata = associated_token_address(&vault_authority, usdc_mint);
    let agent_ata = associated_token_address(agent_wallet, usdc_mint);
    let data = stargaze_anchor::instruction::CloseSession {
        session_id: Pubkey::new_from_array(session_id),
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*caller, true),
            AccountMeta::new_readonly(usdc_config, false),
            AccountMeta::new(session, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(session_vault_ata, false),
            AccountMeta::new(agent_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

// ============ VAULT REGISTRY: PDA helpers ============

pub fn vault_config_pda(provider_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", provider_id.as_ref()], &PROGRAM_ID)
}

// ============ VAULT REGISTRY: instruction builders ============

/// Build the `configure_vault` instruction. Caller is the provider owner.
pub fn ix_configure_vault(
    owner: &Pubkey,
    provider_id: [u8; 32],
    tier: stargaze_anchor::VaultTier,
    on_chain_verifier: Pubkey,
    arweave_cid: [u8; 32],
) -> Instruction {
    let (provider, _) = provider_pda(&provider_id);
    let (vault_config, _) = vault_config_pda(&provider_id);
    let data = stargaze_anchor::instruction::ConfigureVault {
        provider_id,
        tier,
        on_chain_verifier,
        arweave_cid,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(provider, false),
            AccountMeta::new(vault_config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build the `set_vault_auditor_key` instruction. Caller is the provider owner.
pub fn ix_set_vault_auditor_key(
    owner: &Pubkey,
    provider_id: [u8; 32],
    auditor_key: Pubkey,
) -> Instruction {
    let (provider, _) = provider_pda(&provider_id);
    let (vault_config, _) = vault_config_pda(&provider_id);
    let data = stargaze_anchor::instruction::SetVaultAuditorKey {
        provider_id,
        auditor_key,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(provider, false),
            AccountMeta::new(vault_config, false),
        ],
        data,
    }
}

/// Build the `set_vault_buyer_key_rotation_cid` instruction. Caller is the
/// provider owner.
pub fn ix_set_vault_buyer_key_rotation_cid(
    owner: &Pubkey,
    provider_id: [u8; 32],
    cid: [u8; 32],
) -> Instruction {
    let (provider, _) = provider_pda(&provider_id);
    let (vault_config, _) = vault_config_pda(&provider_id);
    let data = stargaze_anchor::instruction::SetVaultBuyerKeyRotationCid {
        provider_id,
        cid,
    }
    .data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(provider, false),
            AccountMeta::new(vault_config, false),
        ],
        data,
    }
}

/// Build the `deactivate_vault` instruction. Caller is the protocol admin
/// (`Config.authority`), NOT the provider owner.
pub fn ix_deactivate_vault(admin: &Pubkey, provider_id: [u8; 32]) -> Instruction {
    let (config, _) = config_pda();
    let (vault_config, _) = vault_config_pda(&provider_id);
    let data = stargaze_anchor::instruction::DeactivateVault { provider_id }.data();
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(vault_config, false),
        ],
        data,
    }
}
