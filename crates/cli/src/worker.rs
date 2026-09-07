use anyhow::Result;
use chrono::{DateTime, Utc};
use ghwork_core::{Event, Item, SyncStats};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

pub enum Cmd {
    Refresh(usize),
    Poll(usize),
    Thread { url: String, repo: String, number: u64 },
}

pub enum Evt {
    Synced { stats: SyncStats, items: Vec<Item> },
    Thread { url: String, events: Vec<Event> },
    Quiet,
    Failed(String),
}

pub struct Worker {
    cmd: Sender<Cmd>,
    pub evt: Receiver<Evt>,
    busy: Arc<AtomicBool>,
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
        let mut sync = ghwork_core::open()?;
        let items = sync.items()?;
        let login = sync.login()?;
        let last_sync = sync.last_sync();
        let interval = sync.poll_interval();

        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (evt_tx, evt_rx) = mpsc::channel::<Evt>();
        let busy = Arc::new(AtomicBool::new(false));

        let flag = busy.clone();
        std::thread::spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                flag.store(true, Ordering::Relaxed);
                if let Cmd::Thread { url, repo, number } = cmd {
                    let evt = match sync.thread(&repo, number) {
                        Ok(events) => Evt::Thread { url, events },
                        Err(e) => Evt::Failed(format!("{e:#}")),
                    };
                    flag.store(false, Ordering::Relaxed);
                    if evt_tx.send(evt).is_err() {
                        break;
                    }
                    continue;
                }
                let res = match cmd {
                    Cmd::Refresh(pages) => sync.refresh(pages),
                    Cmd::Poll(pages) => sync.poll(pages),
                    Cmd::Thread { .. } => unreachable!(),
                };
                let evt = match res {
                    Ok(stats) if stats.skipped => Evt::Quiet,
                    Ok(stats) => match sync.items() {
                        Ok(items) => Evt::Synced { stats, items },
                        Err(e) => Evt::Failed(e.to_string()),
                    },
                    Err(e) => Evt::Failed(format!("{e:#}")),
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
            worker: Worker { cmd: cmd_tx, evt: evt_rx, busy },
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
}
