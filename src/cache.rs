use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use rusqlite::Connection;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let expanded = expand_tilde(path);
        let path = Path::new(&expanded);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path).context("Failed to open cache database")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS seen (torrent_id TEXT PRIMARY KEY)",
            [],
        )?;

        Ok(Cache { conn })
    }

    pub fn is_seen(&self, torrent_id: u64) -> bool {
        let id_str = torrent_id.to_string();
        self.conn
            .query_row(
                "SELECT 1 FROM seen WHERE torrent_id = ?1",
                [&id_str],
                |_| Ok(()),
            )
            .is_ok()
    }

    pub fn mark_seen(&self, torrent_id: u64) -> anyhow::Result<()> {
        let id_str = torrent_id.to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO seen (torrent_id) VALUES (?1)",
            [&id_str],
        )?;
        Ok(())
    }

    pub fn mark_all_seen(&self, ids: &[u64]) -> anyhow::Result<()> {
        for id in ids {
            self.mark_seen(*id)?;
        }
        Ok(())
    }

    pub fn seen_set(&self) -> anyhow::Result<HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT torrent_id FROM seen")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut set = HashSet::new();
        for row in rows {
            set.insert(row?);
        }
        Ok(set)
    }
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    path.to_string()
}
