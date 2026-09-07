use anyhow::Result;
use ghwork_core::{Filter, Item, SyncStats};
use std::sync::mpsc::{self, Receiver, Sender};

pub enum Msg {
    Done(SyncStats),
    Failed(String),
}

pub struct App {
    pub items: Vec<Item>,
    pub view: Vec<usize>,
    pub filter: Filter,
    pub cursor: usize,
    pub me: String,
    pub search: String,
    pub searching: bool,
    pub help: bool,
    pub busy: bool,
    pub status: String,
    pub stats: Option<SyncStats>,
    pub quit: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    pub fn new(items: Vec<Item>, me: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            items,
            view: Vec::new(),
            filter: Filter::Attention,
            cursor: 0,
            me,
            search: String::new(),
            searching: false,
            help: false,
            busy: false,
            status: String::new(),
            stats: None,
            quit: false,
            tx,
            rx,
        };
        app.reindex();
        app
    }

    pub fn reindex(&mut self) {
        let q = self.search.to_lowercase();
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
        self.cursor = self.cursor.min(self.view.len().saturating_sub(1));
    }

    pub fn selected(&self) -> Option<&Item> {
        self.view.get(self.cursor).map(|&i| &self.items[i])
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let last = self.view.len() - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, last as isize) as usize;
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
        if self.busy {
            return;
        }
        self.busy = true;
        self.status = "syncing".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let msg = match ghwork_core::open().and_then(|mut s| s.refresh(pages)) {
                Ok(st) => Msg::Done(st),
                Err(e) => Msg::Failed(e.to_string()),
            };
            let _ = tx.send(msg);
        });
    }

    pub fn drain(&mut self) -> Result<()> {
        while let Ok(msg) = self.rx.try_recv() {
            self.busy = false;
            match msg {
                Msg::Done(st) => {
                    self.items = ghwork_core::open()?.items()?;
                    self.stats = Some(st);
                    self.status = format!("{} items, {} api pts", st.fetched, st.cost);
                    self.reindex();
                }
                Msg::Failed(e) => self.status = format!("sync failed: {e}"),
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
