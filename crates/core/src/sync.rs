use crate::{cache::Cache, github::Client, model::Item};
use anyhow::Result;
use chrono::Utc;

const NOTIF_LM: &str = "notifications_last_modified";
const LAST_SYNC: &str = "last_sync";

pub struct Sync {
    pub client: Client,
    pub cache: Cache,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SyncStats {
    pub fetched: usize,
    pub involved_total: u64,
    pub cost: u64,
    pub remaining: u64,
    pub skipped: bool,
    pub notif_ok: bool,
}

impl Sync {
    pub fn new(client: Client, cache: Cache) -> Self {
        Self { client, cache }
    }

    pub fn items(&self) -> Result<Vec<Item>> {
        self.cache.all()
    }

    pub fn last_sync(&self) -> Option<String> {
        self.cache.get_meta(LAST_SYNC).ok().flatten()
    }

    pub fn refresh(&mut self, pages: usize) -> Result<SyncStats> {
        let got = self.client.fetch(pages, 50)?;
        self.cache.put(&got.items)?;
        self.cache.set_meta(LAST_SYNC, &Utc::now().to_rfc3339())?;
        Ok(SyncStats {
            fetched: got.items.len(),
            involved_total: got.involved_total,
            cost: got.cost,
            remaining: got.remaining,
            skipped: false,
            notif_ok: self.prime_notifications().is_ok(),
        })
    }

    fn prime_notifications(&self) -> Result<()> {
        let (_, lm) = self.client.notifications_changed(None)?;
        if let Some(lm) = lm {
            self.cache.set_meta(NOTIF_LM, &lm)?;
        }
        Ok(())
    }

    pub fn poll(&mut self, pages: usize) -> Result<SyncStats> {
        let lm = self.cache.get_meta(NOTIF_LM)?;
        let (changed, next) = self.client.notifications_changed(lm.as_deref())?;
        if let Some(next) = next {
            self.cache.set_meta(NOTIF_LM, &next)?;
        }
        if !changed {
            return Ok(SyncStats { skipped: true, notif_ok: true, ..Default::default() });
        }
        self.refresh(pages)
    }
}
