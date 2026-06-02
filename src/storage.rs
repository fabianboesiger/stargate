use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::theme::Theme;

/// A previously used login, persisted locally for quick reconnect.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedLogin {
    pub id: i64,
    pub server_url: String,
    pub token: String,
    pub identity: String,
    pub last_used: i64,
}

/// A single SQL execution recorded in the local history.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlHistoryEntry {
    pub id: i64,
    pub db_identity: String,
    pub query: String,
    pub executed_at: i64,
    pub success: bool,
}

/// Local persistence layer backed by SQLite.
///
/// The database file is created automatically (along with its parent
/// directory) in the platform-specific application data directory:
/// - macOS:   `~/Library/Application Support/stargate/stargate.db`
/// - Linux:   `~/.local/share/stargate/stargate.db`
/// - Windows: `%APPDATA%\stargate\stargate.db`
///
/// All operations are synchronous and fast (local file IO) and must never be
/// held across an `.await` point. When the store cannot be opened, the inner
/// connection is `None` and every operation degrades gracefully to a no-op so
/// the application keeps working.
#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Option<Connection>>>,
}

impl Storage {
    /// Open (and if necessary create + initialize) the local store.
    pub fn open() -> Self {
        let conn = match Self::open_inner() {
            Ok(conn) => Some(conn),
            Err(e) => {
                log::error!("Failed to open local storage, persistence disabled: {e}");
                None
            }
        };
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    fn open_inner() -> Result<Connection, String> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create data dir {}: {e}", parent.display()))?;
        }
        log::info!("Opening local storage at {}", path.display());
        let conn = Connection::open(&path).map_err(|e| format!("open db: {e}"))?;
        Self::init_schema(&conn).map_err(|e| format!("init schema: {e}"))?;
        Ok(conn)
    }

    fn db_path() -> Result<PathBuf, String> {
        let dir = dirs::data_dir()
            .ok_or_else(|| "could not resolve application data directory".to_string())?;
        Ok(dir.join("stargate").join("stargate.db"))
    }

    fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS saved_logins (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                server_url  TEXT NOT NULL,
                token       TEXT NOT NULL,
                identity    TEXT NOT NULL,
                last_used   INTEGER NOT NULL,
                UNIQUE(server_url, identity)
            );
            CREATE TABLE IF NOT EXISTS sql_history (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                db_identity TEXT NOT NULL,
                query       TEXT NOT NULL,
                executed_at INTEGER NOT NULL,
                success     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sql_history_db
                ON sql_history(db_identity, executed_at DESC);
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn with_conn<T, F>(&self, default: T, f: F) -> T
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T>,
    {
        let guard = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("Storage mutex poisoned: {e}");
                return default;
            }
        };
        let Some(conn) = guard.as_ref() else {
            return default;
        };
        match f(conn) {
            Ok(value) => value,
            Err(e) => {
                log::error!("Storage operation failed: {e}");
                default
            }
        }
    }

    // ----- Saved logins -------------------------------------------------

    /// All saved logins, most recently used first.
    pub fn list_logins(&self) -> Vec<SavedLogin> {
        self.with_conn(Vec::new(), |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, server_url, token, identity, last_used
                 FROM saved_logins
                 ORDER BY last_used DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(SavedLogin {
                    id: row.get(0)?,
                    server_url: row.get(1)?,
                    token: row.get(2)?,
                    identity: row.get(3)?,
                    last_used: row.get(4)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Insert or update a saved login (keyed by server_url + identity).
    pub fn upsert_login(&self, server_url: &str, token: &str, identity: &str) {
        let now = Self::now();
        self.with_conn((), |conn| {
            conn.execute(
                "INSERT INTO saved_logins (server_url, token, identity, last_used)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(server_url, identity)
                 DO UPDATE SET token = excluded.token, last_used = excluded.last_used",
                rusqlite::params![server_url, token, identity, now],
            )?;
            Ok(())
        });
    }

    /// Update the last-used timestamp for a saved login.
    pub fn touch_login(&self, id: i64) {
        let now = Self::now();
        self.with_conn((), |conn| {
            conn.execute(
                "UPDATE saved_logins SET last_used = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )?;
            Ok(())
        });
    }

    /// Remove a saved login.
    pub fn delete_login(&self, id: i64) {
        self.with_conn((), |conn| {
            conn.execute("DELETE FROM saved_logins WHERE id = ?1", [id])?;
            Ok(())
        });
    }

    // ----- SQL history --------------------------------------------------

    /// Record an executed query for the given database.
    pub fn add_history(&self, db_identity: &str, query: &str, success: bool) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }
        let now = Self::now();
        self.with_conn((), |conn| {
            conn.execute(
                "INSERT INTO sql_history (db_identity, query, executed_at, success)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![db_identity, query, now, success as i64],
            )?;
            Ok(())
        });
    }

    /// Most recent queries for a database, newest first.
    pub fn list_history(&self, db_identity: &str, limit: usize) -> Vec<SqlHistoryEntry> {
        self.with_conn(Vec::new(), |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, db_identity, query, executed_at, success
                 FROM sql_history
                 WHERE db_identity = ?1
                 ORDER BY executed_at DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![db_identity, limit as i64], |row| {
                Ok(SqlHistoryEntry {
                    id: row.get(0)?,
                    db_identity: row.get(1)?,
                    query: row.get(2)?,
                    executed_at: row.get(3)?,
                    success: row.get::<_, i64>(4)? != 0,
                })
            })?;
            rows.collect()
        })
    }

    /// Remove all history for a database.
    pub fn clear_history(&self, db_identity: &str) {
        self.with_conn((), |conn| {
            conn.execute(
                "DELETE FROM sql_history WHERE db_identity = ?1",
                [db_identity],
            )?;
            Ok(())
        });
    }

    // ----- Settings -----------------------------------------------------

    /// Read a raw setting value by key.
    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.with_conn(None, |conn| {
            conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
        })
    }

    /// Insert or update a setting value by key.
    pub fn set_setting(&self, key: &str, value: &str) {
        self.with_conn((), |conn| {
            conn.execute(
                "INSERT INTO settings (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        });
    }

    /// The persisted UI theme, defaulting to [`Theme::default`].
    pub fn get_theme(&self) -> Theme {
        self.get_setting("theme")
            .map(|v| Theme::from_str(&v))
            .unwrap_or_default()
    }

    /// Persist the selected UI theme.
    pub fn set_theme(&self, theme: Theme) {
        self.set_setting("theme", theme.as_str());
    }
}
