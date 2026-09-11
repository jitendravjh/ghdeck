use anyhow::Result;
use chrono::{DateTime, Utc};
use ghdeck_core::{Event, Item, SyncStats};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

pub enum Cmd {
    Refresh(usize),
    Poll(usize),
    Thread { url: String, repo: String, number: u64 },
    Activity { login: String, pages: usize, ticket: u64 },
}

pub enum Evt {
    Synced { stats: SyncStats, items: Vec<Item>, login: String },
    Thread { url: String, events: Vec<Event> },
    Activity { ticket: u64, items: Vec<Item>, total: u64, cost: Option<u64>, since: Option<DateTime<Utc>> },
    ActivityFailed { ticket: u64, error: String },
    Quiet,
    Failed(String),
}

pub struct Worker {
    cmd: Sender<Cmd>,
    pub evt: Receiver<Evt>,
    busy: Arc<AtomicBool>,
    ticket: Arc<AtomicU64>,
}

pub struct Boot {
    pub worker: Worker,
    pub items: Vec<Item>,
    pub login: String,
    pub last_sync: Option<DateTime<Utc>>,
    pub interval: u64,
}

impl Worker {
    pub fn boot() -> Result<Boot> {
        let mut sync = ghdeck_core::open()?;
        let items = sync.items()?;
        let login = sync.login()?;
        let last_sync = sync.last_sync();
        let interval = sync.poll_interval();

        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (evt_tx, evt_rx) = mpsc::channel::<Evt>();
        let busy = Arc::new(AtomicBool::new(false));
        let ticket = Arc::new(AtomicU64::new(0));

        let flag = busy.clone();
        let latest = ticket.clone();
        std::thread::spawn(move || {
            let mut later: VecDeque<Cmd> = VecDeque::new();
            loop {
                let cmd = match later.pop_front() {
                    Some(c) => c,
                    None => match cmd_rx.recv() {
                        Ok(c) => c,
                        Err(_) => break,
                    },
                };
                flag.store(true, Ordering::Relaxed);
                let evt = match cmd {
                    Cmd::Refresh(pages) => {
                        let res = sync.refresh(pages);
                        synced(&sync, res)
                    }
                    Cmd::Poll(pages) => {
                        let res = sync.poll(pages);
                        synced(&sync, res)
                    }
                    Cmd::Thread { url, repo, number } => load_thread(&sync, url, &repo, number),
                    Cmd::Activity { login, pages, ticket } => {
                        let live = || latest.load(Ordering::Relaxed) == ticket;
                        if !live() {
                            flag.store(false, Ordering::Relaxed);
                            continue;
                        }
                        // a lookup takes several pages, so answer conversations in between and park the rest
                        let mut on_page = |items: &[Item], total: u64| {
                            while let Ok(c) = cmd_rx.try_recv() {
                                match c {
                                    Cmd::Thread { url, repo, number } => {
                                        let _ = evt_tx.send(load_thread(&sync, url, &repo, number));
                                    }
                                    other => later.push_back(other),
                                }
                            }
                            let _ = evt_tx.send(Evt::Activity { ticket, items: items.to_vec(), total, cost: None, since: None });
                            live()
                        };
                        match sync.activity(&login, pages, &mut on_page) {
                            Ok(got) => Evt::Activity {
                                ticket,
                                items: got.items,
                                total: got.involved_total,
                                cost: Some(got.cost),
                                since: got.feed_since,
                            },
                            Err(e) => Evt::ActivityFailed { ticket, error: format!("{e:#}") },
                        }
                    }
                };
                flag.store(false, Ordering::Relaxed);
                if evt_tx.send(evt).is_err() {
                    break;
                }
            }
        });

        let ticker = cmd_tx.clone();
        let flag = busy.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(interval));
            if flag.load(Ordering::Relaxed) {
                continue;
            }
            if ticker.send(Cmd::Poll(5)).is_err() {
                break;
            }
        });

        Ok(Boot {
            worker: Worker { cmd: cmd_tx, evt: evt_rx, busy, ticket },
            items,
            login,
            last_sync,
            interval,
        })
    }

    pub fn send(&self, cmd: Cmd) {
        let _ = self.cmd.send(cmd);
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    pub fn look_up(&self, login: String, pages: usize) -> u64 {
        let ticket = self.ticket.fetch_add(1, Ordering::Relaxed) + 1;
        self.send(Cmd::Activity { login, pages, ticket });
        ticket
    }

    pub fn stop_look_up(&self) {
        self.ticket.fetch_add(1, Ordering::Relaxed);
    }
}

fn load_thread(sync: &ghdeck_core::Sync, url: String, repo: &str, number: u64) -> Evt {
    match sync.thread(repo, number) {
        Ok(events) => Evt::Thread { url, events },
        Err(e) => Evt::Failed(format!("{e:#}")),
    }
}

fn synced(sync: &ghdeck_core::Sync, res: Result<SyncStats>) -> Evt {
    match res {
        Ok(stats) if stats.skipped => Evt::Quiet,
        Ok(stats) => match sync.items() {
            Ok(items) => Evt::Synced { stats, items, login: sync.login().unwrap_or_default() },
            Err(e) => Evt::Failed(e.to_string()),
        },
        Err(e) => Evt::Failed(format!("{e:#}")),
    }
}
