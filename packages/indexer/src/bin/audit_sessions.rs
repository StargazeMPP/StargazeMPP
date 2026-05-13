//! `stargaze-audit-sessions` — print recent `session_settled` projection
//! rows joined with the originating `session_opened` row per session.
//! Settlement is one-shot per session, so each row in the output is the
//! final state of a session: the original deposit, the agent wallet that
//! funded it, and the three-way split (to_providers / routing_fee /
//! refund_to_agent) the settle ix produced.
//!
//! The eyeball invariant is `deposit == total_to_providers + routing_fee
//! + refund_to_agent` (modulo any non-routing-fee skim a future ix might
//! introduce). Sessions that the indexer never saw open render `-` in
//! the agent/deposit cells.
//!
//! Reads `DATABASE_URL` from env (or `.env`). Accepts an optional
//! `--limit N` (default 50) to bound the result set.
//!
//! ```text
//! DATABASE_URL=postgres://user:pass@host/db cargo run -p stargaze-indexer \
//!     --bin stargaze-audit-sessions -- --limit 20
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

    // LEFT JOIN LATERAL on the most-recent SessionOpened row per
    // session_id. A session_id is a 32-byte handle minted by the agent
    // and should be unique per session, but LATERAL + ORDER BY slot DESC
    // is the same defensive shape used by audit_vault_proofs and keeps
    // the query robust to any duplicate-emit edge case.
    let rows = sqlx::query(
        r#"
        SELECT
            ss.created_at,
            ss.slot,
            ss.signature,
            ss.session_id,
            ss.total_to_providers,
            ss.routing_fee,
            ss.refund_to_agent,
            so.agent_wallet AS agent_wallet,
            so.deposit      AS deposit
        FROM session_settled ss
        LEFT JOIN LATERAL (
            SELECT agent_wallet, deposit
            FROM session_opened
            WHERE session_id = ss.session_id
            ORDER BY slot DESC
            LIMIT 1
        ) so ON true
        ORDER BY ss.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .context("querying session_settled")?;

    if rows.is_empty() {
        println!("no session_settled rows found");
        return Ok(());
    }

    println!(
        "created_at\tslot\tsession_id\tagent_wallet\tdeposit\ttotal_to_providers\trouting_fee\trefund_to_agent\tsignature"
    );
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let slot: i64 = row.try_get("slot")?;
        let session_id: Vec<u8> = row.try_get("session_id")?;
        let total_to_providers: i64 = row.try_get("total_to_providers")?;
        let routing_fee: i64 = row.try_get("routing_fee")?;
        let refund_to_agent: i64 = row.try_get("refund_to_agent")?;
        let signature: String = row.try_get("signature")?;
        let agent_wallet: Option<Vec<u8>> = row.try_get("agent_wallet")?;
        let deposit: Option<i64> = row.try_get("deposit")?;

        println!(
            "{ts}\t{slot}\t{sid}\t{agent}\t{dep}\t{ttp}\t{rf}\t{rta}\t{sig}",
            ts = created_at.format("%Y-%m-%dT%H:%M:%SZ"),
            slot = slot,
            sid = hex::encode(&session_id),
            agent = hex_or_dash(agent_wallet.as_deref()),
            dep = i64_or_dash(deposit),
            ttp = total_to_providers,
            rf = routing_fee,
            rta = refund_to_agent,
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

fn i64_or_dash(value: Option<i64>) -> String {
    match value {
        Some(v) => v.to_string(),
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
                    "stargaze-audit-sessions [--limit N]\n\
                     \n\
                     Reads DATABASE_URL from env and prints recent session_settled\n\
                     rows joined with the originating session_opened row per session.\n\
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

    #[test]
    fn i64_or_dash_handles_none() {
        assert_eq!(i64_or_dash(None), "-");
    }

    #[test]
    fn i64_or_dash_renders_value() {
        assert_eq!(i64_or_dash(Some(123_456)), "123456");
    }
}
