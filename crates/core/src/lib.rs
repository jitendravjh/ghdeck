pub mod auth;
pub mod cache;
pub mod github;
pub mod model;
pub mod sync;
pub mod thread;

use anyhow::Result;

pub use model::{ago, clip, plural, Ci, Item, Kind, Review, State, Tone};
pub use sync::{Sync, SyncStats};
pub use thread::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Attention,
    Open,
    Mine,
    Reviews,
    All,
}

impl Filter {
    pub const ORDER: [Filter; 5] =
        [Filter::Attention, Filter::Open, Filter::Mine, Filter::Reviews, Filter::All];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Attention => "ATTENTION NEEDED",
            Filter::Open => "OPEN",
            Filter::Mine => "YOURS",
            Filter::Reviews => "TO REVIEW",
            Filter::All => "ALL",
        }
    }

    pub fn keeps(self, it: &Item, me: &str) -> bool {
        match self {
            Filter::Attention => it.needs_attention(),
            Filter::Open => it.is_open(),
            Filter::Mine => it.author == me,
            Filter::Reviews => it.review_requested_of_me,
            Filter::All => true,
        }
    }
}

pub fn open() -> Result<Sync> {
    let client = github::Client::new(auth::token()?);
    let cache = cache::Cache::open(&cache::default_path()?)?;
    Ok(Sync::new(client, cache))
}
