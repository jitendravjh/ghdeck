use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Pr,
    Issue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Open,
    Draft,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Review {
    Approved,
    ChangesRequested,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ci {
    Success,
    Failure,
    Pending,
    Error,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Bad,
    Warn,
    Info,
    Muted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub url: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub kind: Kind,
    pub state: State,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    pub conflicting: bool,
    pub mergeable_unknown: bool,
    pub review: Option<Review>,
    pub ci: Ci,
    pub comments: u64,
    pub reviews_left: u64,
    pub waiting_on: Vec<String>,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
    pub last_actor: Option<String>,
    pub last_action: Option<String>,
    pub review_requested_of_me: bool,
    pub state_reason: Option<String>,
}

impl Item {
    pub fn is_open(&self) -> bool {
        matches!(self.state, State::Open | State::Draft)
    }

    pub fn needs_attention(&self) -> bool {
        self.is_open()
            && (self.conflicting
                || self.review == Some(Review::ChangesRequested)
                || matches!(self.ci, Ci::Failure | Ci::Error)
                || self.review_requested_of_me)
    }

    pub fn slug(&self) -> String {
        format!("{}#{}", self.repo, self.number)
    }

    pub fn activity(&self) -> Option<String> {
        match (&self.last_actor, &self.last_action) {
            (Some(a), Some(b)) => Some(format!("{a} {b}")),
            _ => None,
        }
    }

    pub fn chips(&self) -> Vec<(String, Tone)> {
        let mut out = Vec::new();
        match self.state {
            State::Open => out.push(("open".into(), Tone::Good)),
            State::Draft => out.push(("draft".into(), Tone::Muted)),
            State::Merged => out.push(("merged".into(), Tone::Info)),
            State::Closed => out.push((
                if self.state_reason.as_deref() == Some("NOT_PLANNED") {
                    "not planned".into()
                } else {
                    "closed".into()
                },
                Tone::Bad,
            )),
        }
        if !self.is_open() {
            if self.review == Some(Review::Approved) {
                out.push(("approved".into(), Tone::Good));
            }
            return out;
        }
        if self.review_requested_of_me {
            out.push(("your review".into(), Tone::Warn));
        }
        match self.review {
            Some(Review::Approved) => out.push(("approved".into(), Tone::Good)),
            Some(Review::ChangesRequested) => out.push(("changes requested".into(), Tone::Bad)),
            Some(Review::ReviewRequired) => out.push(("needs review".into(), Tone::Warn)),
            None => {}
        }
        if self.conflicting {
            out.push(("conflict".into(), Tone::Bad));
        }
        match self.ci {
            Ci::Success => out.push(("ci pass".into(), Tone::Good)),
            Ci::Failure => out.push(("ci fail".into(), Tone::Bad)),
            Ci::Error => out.push(("ci error".into(), Tone::Bad)),
            Ci::Pending => out.push(("ci running".into(), Tone::Warn)),
            Ci::None => {}
        }
        if self.reviews_left > 0 && self.review.is_none() {
            out.push(("reviewed".into(), Tone::Info));
        }
        if !self.waiting_on.is_empty() {
            out.push((format!("waiting on {}", self.waiting_on.join(", ")), Tone::Warn));
        }
        out
    }
}

pub fn plural(n: u64, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn ago(then: DateTime<Utc>) -> String {
    let secs = (Utc::now() - then).num_seconds().max(0);
    match secs {
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s if s < 2_592_000 => format!("{}d", s / 86_400),
        s if s < 31_536_000 => format!("{}mo", s / 2_592_000),
        s => format!("{}y", s / 31_536_000),
    }
}
