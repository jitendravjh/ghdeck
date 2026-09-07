use crate::worker::{Boot, Cmd, Evt, Worker};
use anyhow::Result;
use chrono::{DateTime, Utc};
use ghwork_core::{Filter, Item, SyncStats};

pub struct App {
    pub items: Vec<Item>,
    pub view: Vec<usize>,
    pub filter: Filter,
    pub cursor: usize,
    pub me: String,
    pub search: String,
    pub searching: bool,
    pub help: bool,
    pub status: String,
    pub stats: Option<SyncStats>,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_check: Option<DateTime<Utc>>,
    pub interval: u64,
    pub quit: bool,
    worker: Worker,
}

impl App {
    pub fn new(boot: Boot) -> Self {
        let mut app = Self {
            items: boot.items,
            view: Vec::new(),
            filter: Filter::Attention,
            cursor: 0,
            me: boot.login,
            search: String::new(),
            searching: false,
            help: false,
            status: String::new(),
            stats: None,
            last_sync: boot.last_sync,
            last_check: boot.last_sync,
            interval: boot.interval,
            quit: false,
            worker: boot.worker,
        };
        app.reindex();
        app
    }

    pub fn busy(&self) -> bool {
        self.worker.is_busy()
    }

    pub fn reindex(&mut self) {
        let q = self.search.to_lowercase();
        let keep = self.selected().map(|it| it.url.clone());
        self.view = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| self.filter.keeps(it, &self.me))
            .filter(|(_, it)| {
                q.is_empty()
                    || it.title.to_lowercase().contains(&q)
                    || it.repo.to_lowercase().contains(&q)
                    || it.labels.iter().any(|l| l.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
        self.cursor = keep
            .and_then(|url| self.view.iter().position(|&i| self.items[i].url == url))
            .unwrap_or(self.cursor)
            .min(self.view.len().saturating_sub(1));
    }

    pub fn selected(&self) -> Option<&Item> {
        self.view.get(self.cursor).map(|&i| &self.items[i])
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let last = (self.view.len() - 1) as isize;
        self.cursor = (self.cursor as isize).saturating_add(delta).clamp(0, last) as usize;
    }

    pub fn jump_to_end(&mut self) {
        self.cursor = self.view.len().saturating_sub(1);
    }

    pub fn set_filter(&mut self, f: Filter) {
        self.filter = f;
        self.cursor = 0;
        self.reindex();
    }

    pub fn cycle_filter(&mut self, forward: bool) {
        let cur = Filter::ORDER.iter().position(|f| *f == self.filter).unwrap_or(0);
        let n = Filter::ORDER.len();
        let next = if forward { (cur + 1) % n } else { (cur + n - 1) % n };
        self.set_filter(Filter::ORDER[next]);
    }

    pub fn count(&self, f: Filter) -> usize {
        self.items.iter().filter(|it| f.keeps(it, &self.me)).count()
    }

    pub fn refresh(&mut self, pages: usize) {
        self.status = "syncing".into();
        self.worker.send(Cmd::Refresh(pages));
    }

    pub fn drain(&mut self) -> Result<()> {
        while let Ok(evt) = self.worker.evt.try_recv() {
            self.last_check = Some(Utc::now());
            match evt {
                Evt::Synced { stats, items } => {
                    self.items = items;
                    self.stats = Some(stats);
                    self.last_sync = Some(Utc::now());
                    self.status = format!("{} items, {} api pts", stats.fetched, stats.cost);
                    self.reindex();
                }
                Evt::Quiet => self.status.clear(),
                Evt::Failed(e) => self.status = format!("sync failed: {e}"),
            }
        }
        Ok(())
    }

    pub fn open_in_browser(&self) {
        if let Some(it) = self.selected() {
            let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            let _ = std::process::Command::new(cmd).arg(&it.url).spawn();
        }
    }
}
