use crate::model::{HistoryRow, Stats};
use crate::settings::Settings;
use crate::timeutil::{DAY, now};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct Store {
    conn: Mutex<Connection>,
}

/// A history entry about to be written.
pub struct NewHistory<'a> {
    pub action: &'a str,
    pub kind: &'a str,
    pub finding_id: &'a str,
    pub title: &'a str,
    pub path: &'a str,
    pub dest: Option<&'a str>,
    pub bytes: u64,
    pub batch: &'a str,
    pub undoable: bool,
    pub auto: bool,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn memory() -> Self {
        Self::init(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS kv (k TEXT PRIMARY KEY, v TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS decisions (
                 id TEXT PRIMARY KEY, path TEXT NOT NULL, decision TEXT NOT NULL,
                 until INTEGER NOT NULL, at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS first_seen (path TEXT PRIMARY KEY, ts INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS seen (id TEXT PRIMARY KEY, at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 at INTEGER NOT NULL, action TEXT NOT NULL, kind TEXT NOT NULL,
                 finding_id TEXT NOT NULL, title TEXT NOT NULL, path TEXT NOT NULL,
                 dest TEXT, bytes INTEGER NOT NULL, batch TEXT NOT NULL,
                 undoable INTEGER NOT NULL, undone INTEGER NOT NULL DEFAULT 0,
                 auto INTEGER NOT NULL DEFAULT 0);
             CREATE INDEX IF NOT EXISTS history_at ON history(at);
             CREATE TABLE IF NOT EXISTS protected (
                 path TEXT PRIMARY KEY, until INTEGER NOT NULL, at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS verdicts (
                 kind TEXT NOT NULL, ext TEXT NOT NULL, host TEXT NOT NULL,
                 folder TEXT NOT NULL, verdict TEXT NOT NULL, at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS hashes (
                 path TEXT PRIMARY KEY, size INTEGER NOT NULL, mtime INTEGER NOT NULL,
                 kind TEXT NOT NULL, hash INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS hibernated (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, archive TEXT NOT NULL,
                 bytes INTEGER NOT NULL, files INTEGER NOT NULL, at INTEGER NOT NULL,
                 restored INTEGER NOT NULL DEFAULT 0);",
        )?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    // ---- key/value -------------------------------------------------------

    pub fn get(&self, k: &str) -> Option<String> {
        self.conn
            .lock()
            .query_row("SELECT v FROM kv WHERE k=?1", [k], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    }

    pub fn set(&self, k: &str, v: &str) {
        let _ = self.conn.lock().execute(
            "INSERT INTO kv(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
            params![k, v],
        );
    }

    pub fn get_i64(&self, k: &str) -> i64 {
        self.get(k).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn settings(&self) -> Settings {
        self.get("settings")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &Settings) {
        if let Ok(json) = serde_json::to_string(s) {
            self.set("settings", &json);
        }
    }

    // ---- decisions -------------------------------------------------------

    pub fn set_decision(&self, id: &str, path: &str, decision: &str, until: i64) {
        let _ = self.conn.lock().execute(
            "INSERT INTO decisions(id,path,decision,until,at) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET decision=excluded.decision, until=excluded.until, at=excluded.at",
            params![id, path, decision, until, now()],
        );
    }

    pub fn clear_decision(&self, id: &str) {
        let _ = self
            .conn
            .lock()
            .execute("DELETE FROM decisions WHERE id=?1", [id]);
    }

    /// Finding ids that must not be shown: kept forever, or snoozed until later.
    pub fn suppressed(&self) -> HashSet<String> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT id FROM decisions WHERE decision='keep' OR (decision='snooze' AND until > ?1)",
        ) {
            Ok(s) => s,
            Err(_) => return HashSet::new(),
        };
        stmt.query_map([now()], |r| r.get::<_, String>(0))
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    // ---- first seen (watcher) ---------------------------------------------

    pub fn mark_first_seen(&self, path: &str, ts: i64) {
        let _ = self.conn.lock().execute(
            "INSERT OR IGNORE INTO first_seen(path, ts) VALUES(?1, ?2)",
            params![path.to_lowercase(), ts],
        );
    }

    pub fn first_seen_map(&self) -> HashMap<String, i64> {
        let conn = self.conn.lock();
        let Ok(mut stmt) = conn.prepare("SELECT path, ts FROM first_seen") else {
            return HashMap::new();
        };
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    // ---- seen findings (for "new" detection) ----------------------------

    /// Records the ids and returns the ones never seen before.
    pub fn mark_seen(&self, ids: &[String]) -> HashSet<String> {
        let mut conn = self.conn.lock();
        let mut fresh = HashSet::new();
        let ts = now();
        if let Ok(tx) = conn.transaction() {
            {
                let Ok(mut stmt) = tx.prepare("INSERT OR IGNORE INTO seen(id, at) VALUES(?1, ?2)")
                else {
                    return fresh;
                };
                for id in ids {
                    if let Ok(n) = stmt.execute(params![id, ts])
                        && n > 0
                    {
                        fresh.insert(id.clone());
                    }
                }
            }
            let _ = tx.commit();
        }
        fresh
    }

    // ---- history ----------------------------------------------------------

    pub fn add_history(&self, h: NewHistory) -> i64 {
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO history(at,action,kind,finding_id,title,path,dest,bytes,batch,undoable,auto)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                now(),
                h.action,
                h.kind,
                h.finding_id,
                h.title,
                h.path,
                h.dest,
                h.bytes as i64,
                h.batch,
                h.undoable,
                h.auto
            ],
        );
        conn.last_insert_rowid()
    }

    fn row(r: &rusqlite::Row) -> rusqlite::Result<HistoryRow> {
        Ok(HistoryRow {
            id: r.get(0)?,
            at: r.get(1)?,
            action: r.get(2)?,
            kind: r.get(3)?,
            finding_id: r.get(4)?,
            title: r.get(5)?,
            path: r.get(6)?,
            dest: r.get(7)?,
            bytes: r.get::<_, i64>(8)? as u64,
            batch: r.get(9)?,
            undoable: r.get(10)?,
            undone: r.get(11)?,
            auto: r.get(12)?,
        })
    }

    const COLS: &'static str =
        "id,at,action,kind,finding_id,title,path,dest,bytes,batch,undoable,undone,auto";

    pub fn history(&self, limit: u32) -> Vec<HistoryRow> {
        let conn = self.conn.lock();
        let sql = format!(
            "SELECT {} FROM history ORDER BY id DESC LIMIT ?1",
            Self::COLS
        );
        let Ok(mut stmt) = conn.prepare(&sql) else {
            return vec![];
        };
        stmt.query_map([limit], Self::row)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    pub fn history_by_batch(&self, batch: &str) -> Vec<HistoryRow> {
        let conn = self.conn.lock();
        let sql = format!(
            "SELECT {} FROM history WHERE batch=?1 ORDER BY id DESC",
            Self::COLS
        );
        let Ok(mut stmt) = conn.prepare(&sql) else {
            return vec![];
        };
        stmt.query_map([batch], Self::row)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    pub fn history_get(&self, id: i64) -> Option<HistoryRow> {
        let conn = self.conn.lock();
        let sql = format!("SELECT {} FROM history WHERE id=?1", Self::COLS);
        conn.query_row(&sql, [id], Self::row)
            .optional()
            .ok()
            .flatten()
    }

    pub fn mark_undone(&self, id: i64) {
        let _ = self
            .conn
            .lock()
            .execute("UPDATE history SET undone=1 WHERE id=?1", [id]);
    }

    pub fn stats(&self) -> Stats {
        let conn = self.conn.lock();
        let freeing = "action IN ('recycle','delete','empty') AND undone=0";
        let q = |sql: &str, p: i64| -> i64 {
            conn.query_row(sql, [p], |r| r.get::<_, Option<i64>>(0))
                .ok()
                .flatten()
                .unwrap_or(0)
        };
        let week_ago = now() - 7 * DAY;
        let mut st = Stats {
            freed_total: q(
                &format!("SELECT SUM(bytes) FROM history WHERE {freeing} AND ?1=?1"),
                0,
            ) as u64,
            freed_week: q(
                &format!("SELECT SUM(bytes) FROM history WHERE {freeing} AND at>=?1"),
                week_ago,
            ) as u64,
            tidied_total: q(
                "SELECT COUNT(*) FROM history WHERE action IN ('archive','sweep') AND undone=0 AND ?1=?1",
                0,
            ) as u64,
            actions_total: q("SELECT COUNT(*) FROM history WHERE undone=0 AND ?1=?1", 0) as u64,
            kept_total: q(
                "SELECT COUNT(*) FROM history WHERE action='keep' AND undone=0 AND ?1=?1",
                0,
            ) as u64,
            streak_days: 0,
        };
        // Consecutive days (ending today or yesterday) with at least one manual action.
        if let Ok(mut stmt) = conn.prepare(
            "SELECT DISTINCT (at / 86400) AS d FROM history WHERE auto=0 AND undone=0 ORDER BY d DESC LIMIT 400",
        ) {
            let days: Vec<i64> = stmt
                .query_map([], |r| r.get(0))
                .map(|rows| rows.flatten().collect())
                .unwrap_or_default();
            let today = now() / DAY;
            let mut expect = today;
            for (i, d) in days.iter().enumerate() {
                if i == 0 && *d == today - 1 {
                    expect = today - 1;
                }
                if *d == expect {
                    st.streak_days += 1;
                    expect -= 1;
                } else {
                    break;
                }
            }
        }
        st
    }
}

/// A path the user asked Dusty to leave alone.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Protected {
    pub path: String,
    /// 0 = forever.
    pub until: i64,
    pub at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hibernated {
    pub id: i64,
    pub project: String,
    pub archive: String,
    pub bytes: u64,
    pub files: u64,
    pub at: i64,
    pub restored: bool,
}

/// Keep/remove tallies for a feature (extension, origin host...).
#[derive(Debug, Clone, Copy, Default)]
pub struct Tally {
    pub keep: u32,
    pub remove: u32,
}

impl Store {
    // ---- protection -------------------------------------------------------

    pub fn protect(&self, path: &str, until: i64) {
        let _ = self.conn.lock().execute(
            "INSERT INTO protected(path, until, at) VALUES(?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET until=excluded.until, at=excluded.at",
            params![path.trim_end_matches('\\').to_lowercase(), until, now()],
        );
    }

    pub fn unprotect(&self, path: &str) {
        let _ = self
            .conn
            .lock()
            .execute("DELETE FROM protected WHERE path=?1", [path.to_lowercase()]);
    }

    pub fn protected(&self) -> Vec<Protected> {
        let conn = self.conn.lock();
        let Ok(mut stmt) = conn.prepare(
            "SELECT path, until, at FROM protected WHERE until=0 OR until>?1 ORDER BY at DESC",
        ) else {
            return vec![];
        };
        stmt.query_map([now()], |r| {
            Ok(Protected {
                path: r.get(0)?,
                until: r.get(1)?,
                at: r.get(2)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    // ---- learning ---------------------------------------------------------

    pub fn add_verdict(&self, kind: &str, ext: &str, host: &str, folder: &str, verdict: &str) {
        let _ = self.conn.lock().execute(
            "INSERT INTO verdicts(kind, ext, host, folder, verdict, at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![kind, ext, host, folder, verdict, now()],
        );
    }

    /// Tallies keyed by "ext:<kind>:<ext>", "host:<host>", "folder:<folder>".
    pub fn tallies(&self) -> HashMap<String, Tally> {
        let conn = self.conn.lock();
        let mut out: HashMap<String, Tally> = HashMap::new();
        let Ok(mut stmt) = conn.prepare("SELECT kind, ext, host, folder, verdict FROM verdicts")
        else {
            return out;
        };
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        });
        if let Ok(rows) = rows {
            for (kind, ext, host, folder, verdict) in rows.flatten() {
                let mut keys = vec![];
                if !ext.is_empty() {
                    keys.push(format!("ext:{kind}:{ext}"));
                }
                if !host.is_empty() {
                    keys.push(format!("host:{host}"));
                }
                if !folder.is_empty() {
                    keys.push(format!("folder:{folder}"));
                }
                for key in keys {
                    let t = out.entry(key).or_default();
                    if verdict == "keep" {
                        t.keep += 1
                    } else {
                        t.remove += 1
                    }
                }
            }
        }
        out
    }

    // ---- hash cache -------------------------------------------------------

    pub fn hashes(&self, kind: &str) -> HashMap<String, (u64, i64, u64)> {
        let conn = self.conn.lock();
        let Ok(mut stmt) = conn.prepare("SELECT path, size, mtime, hash FROM hashes WHERE kind=?1")
        else {
            return HashMap::new();
        };
        stmt.query_map([kind], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)? as u64,
                ),
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    pub fn put_hashes(&self, kind: &str, rows: &[(String, u64, i64, u64)]) {
        let mut conn = self.conn.lock();
        if let Ok(tx) = conn.transaction() {
            if let Ok(mut stmt) = tx.prepare(
                "INSERT INTO hashes(path,size,mtime,kind,hash) VALUES(?1,?2,?3,?4,?5)
                 ON CONFLICT(path) DO UPDATE SET size=excluded.size, mtime=excluded.mtime, kind=excluded.kind, hash=excluded.hash",
            ) {
                for (p, size, mtime, h) in rows {
                    let _ = stmt.execute(params![p, *size as i64, mtime, kind, *h as i64]);
                }
            }
            let _ = tx.commit();
        }
    }

    // ---- hibernated projects ---------------------------------------------

    pub fn add_hibernated(&self, project: &str, archive: &str, bytes: u64, files: u64) -> i64 {
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO hibernated(project, archive, bytes, files, at) VALUES(?1,?2,?3,?4,?5)",
            params![project, archive, bytes as i64, files as i64, now()],
        );
        conn.last_insert_rowid()
    }

    pub fn hibernated(&self) -> Vec<Hibernated> {
        let conn = self.conn.lock();
        let Ok(mut stmt) =
            conn.prepare("SELECT id, project, archive, bytes, files, at, restored FROM hibernated ORDER BY at DESC")
        else {
            return vec![];
        };
        stmt.query_map([], |r| {
            Ok(Hibernated {
                id: r.get(0)?,
                project: r.get(1)?,
                archive: r.get(2)?,
                bytes: r.get::<_, i64>(3)? as u64,
                files: r.get::<_, i64>(4)? as u64,
                at: r.get(5)?,
                restored: r.get(6)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    pub fn mark_restored(&self, archive: &str) {
        let _ = self.conn.lock().execute(
            "UPDATE hibernated SET restored=1 WHERE archive=?1",
            [archive],
        );
    }

    /// Wipes everything except settings (developer mode).
    pub fn reset_all(&self) {
        let _ = self.conn.lock().execute_batch(
            "DELETE FROM kv WHERE k <> 'settings'; DELETE FROM decisions; DELETE FROM first_seen; DELETE FROM seen;
             DELETE FROM history; DELETE FROM protected; DELETE FROM verdicts; DELETE FROM hashes;",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_and_seen() {
        let s = Store::memory();
        s.set_decision("a", "C:\\x", "keep", 0);
        s.set_decision("b", "C:\\y", "snooze", now() + 100);
        s.set_decision("c", "C:\\z", "snooze", now() - 100);
        let sup = s.suppressed();
        assert!(sup.contains("a") && sup.contains("b") && !sup.contains("c"));

        let fresh = s.mark_seen(&["x".into(), "y".into()]);
        assert_eq!(fresh.len(), 2);
        let fresh = s.mark_seen(&["x".into(), "z".into()]);
        assert_eq!(fresh.len(), 1);
        assert!(fresh.contains("z"));
    }

    #[test]
    fn history_and_stats() {
        let s = Store::memory();
        let id = s.add_history(NewHistory {
            action: "recycle",
            kind: "installer",
            finding_id: "f",
            title: "setup.exe",
            path: "C:\\setup.exe",
            dest: None,
            bytes: 1000,
            batch: "b1",
            undoable: true,
            auto: false,
        });
        assert_eq!(s.stats().freed_total, 1000);
        assert_eq!(s.stats().streak_days, 1);
        s.mark_undone(id);
        assert_eq!(s.stats().freed_total, 0);
        assert!(s.history_get(id).unwrap().undone);
    }
}
