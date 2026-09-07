use crate::model::Item;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Cache {
    conn: Connection,
}

pub fn default_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "ghwork")
        .context("could not resolve a data directory")?;
    let dir = dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("ghwork.db"))
}

impl Cache {
    pub fn open(path: &PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "pragma journal_mode = wal;
             create table if not exists items (
               url text primary key,
               updated_at text not null,
               json text not null
             );
             create index if not exists items_updated on items(updated_at desc);
             create table if not exists meta (key text primary key, value text not null);",
        )?;
        Ok(Self { conn })
    }

    pub fn put(&mut self, items: &[Item]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut st = tx.prepare(
                "insert into items(url, updated_at, json) values (?1, ?2, ?3)
                 on conflict(url) do update set updated_at = excluded.updated_at, json = excluded.json",
            )?;
            for it in items {
                st.execute(params![it.url, it.updated_at.to_rfc3339(), serde_json::to_string(it)?])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<Item>> {
        let mut st = self.conn.prepare("select json from items order by updated_at desc")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            if let Ok(it) = serde_json::from_str(&r?) {
                out.push(it);
            }
        }
        Ok(out)
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut st = self.conn.prepare("select value from meta where key = ?1")?;
        let mut rows = st.query([key])?;
        Ok(match rows.next()? {
            Some(r) => Some(r.get(0)?),
            None => None,
        })
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "insert into meta(key, value) values (?1, ?2)
             on conflict(key) do update set value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}
