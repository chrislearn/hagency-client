//! Console change feed (#26): a bounded read of the tables the console's
//! live pages render — dispatches, tasks, alerts — reduced to one JSON
//! fingerprint per category. TS parity: the retained `/api/router/events`
//! sequence (router/src/store.ts:3811 `eventsAfter`: seq, watermark, gap)
//! is the contract the SSE route re-broadcasts; the fingerprint itself is
//! the native equivalent of the retained store's `snapshot()` (:3655)
//! freshness question: "did the page's rows change since I last looked".
//!
//! No new table: the feed is a pure read over existing rows, so a stopped
//! or absent stream can never wedge a writer. `stale` is true when the
//! caller's category version trails the current one — the SSE route then
//! emits that category and the page refetches its own bounded read, the
//! same division the retained dashboard used (event says "changed", page
//! reads its own data).
use crate::Error;
use rusqlite::Connection;
use serde_json::json;

/// One category's fingerprint: the row count and a digest of the rows'
/// changing columns, so a state transition with no count change still
/// reads as a change.
fn category(
    db: &Connection,
    counter: &str,
) -> Result<(i64, String), Error> {
    let (count, digest): (i64, String) =
        db.query_row(counter, [], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok((count, digest))
}

impl super::DomainRepository {
    /// The console feed: `{agents: {version, count}, tasks: …, alerts: …}`
    /// plus a single `version` digest over all three, so a client that
    /// keeps only one cursor still learns that SOMETHING changed.
    pub fn console_feed(&self) -> Result<serde_json::Value, Error> {
        let dispatches = category(&self.db,
            "SELECT COUNT(*),COALESCE((SELECT group_concat(d.id||':'||d.state,'|') FROM \
             (SELECT id,state FROM runner_dispatches ORDER BY id LIMIT 200) d),'')")?;
        let tasks = category(&self.db,
            "SELECT COUNT(*),COALESCE((SELECT group_concat(t.id||':'||json_extract(t.config,'$.status'),'|') FROM \
             (SELECT id,config FROM canonical_tasks ORDER BY id LIMIT 200) t),'')")?;
        let alerts = category(&self.db,
            "SELECT COUNT(*),COALESCE((SELECT group_concat(dedupe_key||':'||CAST(last_seen_ms AS TEXT),'|') FROM \
             (SELECT dedupe_key,last_seen_ms FROM ceiling_alerts ORDER BY dedupe_key LIMIT 200) a),'')")?;
        let agents = hagency_core::canonical::digest(&json!([
            "agents",
            dispatches.0,
            &dispatches.1,
        ]))?;
        let tasksv = hagency_core::canonical::digest(&json!([
            "tasks",
            tasks.0,
            &tasks.1,
        ]))?;
        let alertsv = hagency_core::canonical::digest(&json!([
            "alerts",
            alerts.0,
            &alerts.1,
        ]))?;
        let version = hagency_core::canonical::digest(&json!([
            &agents, &tasksv, &alertsv,
        ]))?;
        Ok(json!({
            "version": version,
            "agents": {"version": agents, "count": dispatches.0},
            "tasks": {"version": tasksv, "count": tasks.0},
            "alerts": {"version": alertsv, "count": alerts.0},
        }))
    }
}
