//! `stargaze-audit-vouchers` — print recent `voucher_settled` projection
//! rows joined with the latest `provider_registered` row per provider.
//! Each voucher carries the monotonically-growing `cumulative_amount`
//! settled-to-provider for its `(session_id, provider_id)` pair, so the
//! emitted table doubles as a per-session running-total tape.
//!
//! Reads `DATABASE_URL` from env (or `.env`). Accepts an optional
//! `--limit N` (default 50) to bound the result set.
//!
//! ```text
//! DATABASE_URL=postgres://user:pass@host/db cargo run -p stargaze-indexer \
//!     --bin stargaze-audit-vouchers -- --limit 20
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
    // provider_id so a voucher whose provider has not been seen by the
    // indexer still renders with a blank owner cell.
    let rows = sqlx::query(
        r#"
        SELECT
            vs.created_at,
            vs.slot,
            vs.signature,
            vs.session_id,
            vs.provider_id,
            vs.nonce,
            vs.delta,
            vs.cumulative_amount,
            vs.to_provider,
            vs.fee,
            pr.owner AS provider_owner
        FROM voucher_settled vs
        LEFT JOIN LATERAL (
            SELECT owner
            FROM provider_registered
            WHERE provider_id = vs.provider_id
            ORDER BY slot DESC
            LIMIT 1
        ) pr ON true
        ORDER BY vs.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .context("querying voucher_settled")?;

    if rows.is_empty() {
        println!("no voucher_settled rows found");
        return Ok(());
    }

    println!(
        "created_at\tslot\tsession_id\tprovider_id\tprovider_owner\tnonce\tdelta\tcumulative_amount\tto_provider\tfee\tsignature"
    );
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let slot: i64 = row.try_get("slot")?;
        let session_id: Vec<u8> = row.try_get("session_id")?;
        let provider_id: Vec<u8> = row.try_get("provider_id")?;
        let nonce: i64 = row.try_get("nonce")?;
        let delta: i64 = row.try_get("delta")?;
        let cumulative_amount: i64 = row.try_get("cumulative_amount")?;
        let to_provider: i64 = row.try_get("to_provider")?;
        let fee: i64 = row.try_get("fee")?;
        let signature: String = row.try_get("signature")?;
        let provider_owner: Option<Vec<u8>> = row.try_get("provider_owner")?;

        println!(
            "{ts}\t{slot}\t{sid}\t{pid}\t{owner}\t{nonce}\t{delta}\t{cum}\t{to_p}\t{fee}\t{sig}",
            ts = created_at.format("%Y-%m-%dT%H:%M:%SZ"),
            slot = slot,
            sid = hex::encode(&session_id),
            pid = hex::encode(&provider_id),
            owner = hex_or_dash(provider_owner.as_deref()),
            nonce = nonce,
            delta = delta,
            cum = cumulative_amount,
            to_p = to_provider,
            fee = fee,
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
                    "stargaze-audit-vouchers [--limit N]\n\
                     \n\
                     Reads DATABASE_URL from env and prints recent voucher_settled\n\
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
