//! `stargaze-audit-vault-proofs` — print recent `vault_proof_verified`
//! projection rows joined with the latest `provider_registered` row per
//! provider. One-liner audit surface: which providers have submitted
//! vault proofs recently, who owns them, what their category hash is.
//!
//! Reads `DATABASE_URL` from env (or `.env`). Accepts an optional
//! `--limit N` (default 50) to bound the result set.
//!
//! ```text
//! DATABASE_URL=postgres://user:pass@host/db cargo run -p stargaze-indexer \
//!     --bin stargaze-audit-vault-proofs -- --limit 20
//! ```

use std::env;

use anyhow::{anyhow, bail, Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 1_000;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let limit = parse_limit(env::args().skip(1))?;

    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL must be set to query the projection database")?;

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .context("connecting to Postgres")?;

    // LEFT JOIN LATERAL on the most-recent ProviderRegistered row per
    // provider_id so a provider that has not been seen by the indexer
    // still shows the proof with empty owner/category cells. Ordering by
    // slot DESC inside the lateral picks the canonical (latest) registration.
    let rows = sqlx::query(
        r#"
        SELECT
            vpv.created_at,
            vpv.slot,
            vpv.signature,
            vpv.provider_id,
            vpv.tier,
            vpv.signals_hash,
            vpv.submitter,
            vpv.on_chain_slot,
            pr.owner          AS provider_owner,
            pr.category_hash  AS provider_category
        FROM vault_proof_verified vpv
        LEFT JOIN LATERAL (
            SELECT owner, category_hash
            FROM provider_registered
            WHERE provider_id = vpv.provider_id
            ORDER BY slot DESC
            LIMIT 1
        ) pr ON true
        ORDER BY vpv.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .context("querying vault_proof_verified")?;

    if rows.is_empty() {
        println!("no vault_proof_verified rows found");
        return Ok(());
    }

    println!(
        "created_at\tslot\ttier\tprovider_id\tprovider_owner\tcategory_hash\tsignals_hash\tsubmitter\ton_chain_slot\tsignature"
    );
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let slot: i64 = row.try_get("slot")?;
        let tier: i16 = row.try_get("tier")?;
        let provider_id: Vec<u8> = row.try_get("provider_id")?;
        let signals_hash: Vec<u8> = row.try_get("signals_hash")?;
        let submitter: Vec<u8> = row.try_get("submitter")?;
        let on_chain_slot: i64 = row.try_get("on_chain_slot")?;
        let signature: String = row.try_get("signature")?;
        let provider_owner: Option<Vec<u8>> = row.try_get("provider_owner")?;
        let provider_category: Option<Vec<u8>> = row.try_get("provider_category")?;

        println!(
            "{ts}\t{slot}\t{tier}\t{pid}\t{owner}\t{cat}\t{sig_hash}\t{submitter}\t{ocs}\t{sig}",
            ts = created_at.format("%Y-%m-%dT%H:%M:%SZ"),
            slot = slot,
            tier = tier,
            pid = hex::encode(&provider_id),
            owner = hex_or_dash(provider_owner.as_deref()),
            cat = hex_or_dash(provider_category.as_deref()),
            sig_hash = hex::encode(&signals_hash),
            submitter = hex::encode(&submitter),
            ocs = on_chain_slot,
            sig = if signature.is_empty() { "-" } else { &signature },
        );
    }

    Ok(())
}

fn hex_or_dash(bytes: Option<&[u8]>) -> String {
    match bytes {
        Some(b) => hex::encode(b),
        None => "-".to_string(),
    }
}

fn parse_limit<I: IntoIterator<Item = String>>(args: I) -> Result<i64> {
    let mut iter = args.into_iter();
    let mut limit: i64 = DEFAULT_LIMIT;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--limit" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--limit requires a value"))?;
                limit = value
                    .parse::<i64>()
                    .with_context(|| format!("--limit must be an integer, got `{value}`"))?;
            }
            "-h" | "--help" => {
                println!(
                    "stargaze-audit-vault-proofs [--limit N]\n\
                     \n\
                     Reads DATABASE_URL from env and prints recent vault_proof_verified\n\
                     rows joined with the latest provider_registered row per provider.\n\
                     N defaults to {DEFAULT_LIMIT}, capped at {MAX_LIMIT}."
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    if !(1..=MAX_LIMIT).contains(&limit) {
        bail!("--limit must be in [1, {MAX_LIMIT}], got {limit}");
    }
    Ok(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_limit_defaults_when_no_args() {
        assert_eq!(parse_limit(Vec::<String>::new()).unwrap(), DEFAULT_LIMIT);
    }

    #[test]
    fn parse_limit_accepts_value() {
        let args = vec!["--limit".into(), "12".into()];
        assert_eq!(parse_limit(args).unwrap(), 12);
    }

    #[test]
    fn parse_limit_rejects_zero() {
        let args = vec!["--limit".into(), "0".into()];
        assert!(parse_limit(args).is_err());
    }

    #[test]
    fn parse_limit_rejects_above_max() {
        let args = vec!["--limit".into(), format!("{}", MAX_LIMIT + 1)];
        assert!(parse_limit(args).is_err());
    }

    #[test]
    fn parse_limit_rejects_unknown_arg() {
        let args = vec!["--bogus".into()];
        assert!(parse_limit(args).is_err());
    }

    #[test]
    fn parse_limit_rejects_missing_value() {
        let args = vec!["--limit".into()];
        assert!(parse_limit(args).is_err());
    }

    #[test]
    fn hex_or_dash_handles_none() {
        assert_eq!(hex_or_dash(None), "-");
    }

    #[test]
    fn hex_or_dash_encodes_bytes() {
        assert_eq!(hex_or_dash(Some(&[0xde, 0xad])), "dead");
    }
}
