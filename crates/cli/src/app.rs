use crate::worker::{Boot, Cmd, Evt, Worker};
use anyhow::Result;
use chrono::{DateTime, Utc};
use ghwork_core::{Event, Filter, Item, SyncStats};
use std::collections::HashMap;

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
    pub detail: bool,
    pub scroll: u16,
    pub show_bots: bool,
    pub threads: HashMap<String, Vec<Event>>,
    pub loading: Option<String>,
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
            detail: false,
            scroll: 0,
            show_bots: false,
            threads: HashMap::new(),
            loading: None,
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

    pub fn open_detail(&mut self) {
        let Some(it) = self.selected() else { return };
        let (url, repo, number) = (it.url.clone(), it.repo.clone(), it.number);
        self.detail = true;
        self.scroll = 0;
        if !self.threads.contains_key(&url) {
            self.loading = Some(url.clone());
            self.worker.send(Cmd::Thread { url, repo, number });
        }
    }

    pub fn close_detail(&mut self) {
        self.detail = false;
        self.scroll = 0;
    }

    pub fn reload_thread(&mut self) {
        if let Some(it) = self.selected() {
            let (url, repo, number) = (it.url.clone(), it.repo.clone(), it.number);
            self.threads.remove(&url);
            self.loading = Some(url.clone());
            self.worker.send(Cmd::Thread { url, repo, number });
        }
    }

    pub fn thread(&self) -> Option<&Vec<Event>> {
        self.selected().and_then(|it| self.threads.get(&it.url))
    }

    pub fn thread_loading(&self) -> bool {
        match (&self.loading, self.selected()) {
            (Some(url), Some(it)) => *url == it.url,
            _ => false,
        }
    }

    pub fn scroll_by(&mut self, delta: i32) {
        self.scroll = (self.scroll as i32 + delta).max(0) as u16;
    }

    pub fn refresh(&mut self, pages: usize) {
        self.status = "syncing".into();
        self.worker.send(Cmd::Refresh(pages));
    }

    pub fn drain(&mut self) -> Result<()> {
        while let Ok(evt) = self.worker.evt.try_recv() {
            self.last_check = Some(Utc::now());
            match evt {
                Evt::Synced { stats, items, login } => {
                    if !login.is_empty() {
                        self.me = login;
                    }
                    self.items = items;
                    self.stats = Some(stats);
                    self.last_sync = Some(Utc::now());
                    self.status = format!("{} items, {} api pts", stats.fetched, stats.cost);
                    self.reindex();
                }
                Evt::Thread { url, events } => {
                    if self.loading.as_deref() == Some(url.as_str()) {
                        self.loading = None;
                    }
                    self.threads.insert(url, events);
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
