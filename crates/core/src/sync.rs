use crate::{cache::Cache, github::Client, model::Item};
use anyhow::Result;
use chrono::{DateTime, Utc};

const NOTIF_LM: &str = "notifications_last_modified";
const LAST_SYNC: &str = "last_sync";
const LOGIN: &str = "login";
const POLL_INTERVAL: &str = "poll_interval";

pub const DEFAULT_POLL_SECS: u64 = 60;

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

    pub fn last_sync(&self) -> Option<DateTime<Utc>> {
        self.cache
            .get_meta(LAST_SYNC)
            .ok()
            .flatten()
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc))
    }

    pub fn poll_interval(&self) -> u64 {
        if let Some(secs) = std::env::var("GHWORK_POLL_SECS").ok().and_then(|v| v.parse::<u64>().ok()) {
            return secs.max(10);
        }
        self.cache
            .get_meta(POLL_INTERVAL)
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_POLL_SECS)
            .max(30)
    }

    pub fn login(&self) -> Result<String> {
        if let Some(login) = self.cache.get_meta(LOGIN)? {
            if !login.is_empty() {
                return Ok(login);
            }
        }
        let from_gh = std::process::Command::new("gh")
            .args(["api", "user", "-q", ".login"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !from_gh.is_empty() {
            self.cache.set_meta(LOGIN, &from_gh)?;
        }
        Ok(from_gh)
    }

    pub fn refresh(&mut self, pages: usize) -> Result<SyncStats> {
        let got = self.client.fetch(pages, 50)?;
        self.cache.put(&got.items)?;
        self.cache.set_meta(LAST_SYNC, &Utc::now().to_rfc3339())?;
        if !got.login.is_empty() {
            self.cache.set_meta(LOGIN, &got.login)?;
        }
        Ok(SyncStats {
            fetched: got.items.len(),
            involved_total: got.involved_total,
            cost: got.cost,
            remaining: got.remaining,
            skipped: false,
            notif_ok: self.watermark(None).is_ok(),
        })
    }

    fn watermark(&self, last_modified: Option<&str>) -> Result<bool> {
        let poll = self.client.notifications_changed(last_modified)?;
        if let Some(lm) = poll.last_modified {
            self.cache.set_meta(NOTIF_LM, &lm)?;
        }
        if let Some(secs) = poll.interval {
            self.cache.set_meta(POLL_INTERVAL, &secs.to_string())?;
        }
        Ok(poll.changed)
    }

    pub fn thread(&self, repo: &str, number: u64) -> Result<Vec<crate::thread::Event>> {
        self.client.thread(repo, number)
    }

    pub fn poll(&mut self, pages: usize) -> Result<SyncStats> {
        let lm = self.cache.get_meta(NOTIF_LM)?;
        if !self.watermark(lm.as_deref())? {
            return Ok(SyncStats { skipped: true, notif_ok: true, ..Default::default() });
        }
        self.refresh(pages)
    }
}
