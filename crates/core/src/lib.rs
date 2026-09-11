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
    Authored,
    Mentioned,
}

impl Filter {
    pub const ORDER: [Filter; 5] =
        [Filter::Attention, Filter::Open, Filter::Mine, Filter::Reviews, Filter::All];
    pub const PERSON: [Filter; 4] = [Filter::All, Filter::Authored, Filter::Mentioned, Filter::Open];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Attention => "ATTENTION NEEDED",
            Filter::Open => "OPEN",
            Filter::Mine => "YOURS",
            Filter::Reviews => "TO REVIEW",
            Filter::All => "ALL",
            Filter::Authored => "AUTHORED",
            Filter::Mentioned => "MENTIONED",
        }
    }

    pub fn keeps(self, it: &Item, me: &str) -> bool {
        match self {
            Filter::Attention => it.needs_attention(),
            Filter::Open => it.is_open(),
            Filter::Mine | Filter::Authored => it.author.eq_ignore_ascii_case(me),
            Filter::Reviews => it.review_requested_of_me,
            Filter::All => true,
            Filter::Mentioned => it.mentioned,
        }
    }
}

pub fn clean_login(input: &str) -> Option<String> {
    let s = input.trim().trim_start_matches('@');
    let ok = !s.is_empty()
        && s.len() <= 39
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    ok.then(|| s.to_string())
}

pub fn open() -> Result<Sync> {
    let client = github::Client::new(auth::token()?);
    let cache = cache::Cache::open(&cache::default_path()?)?;
    Ok(Sync::new(client, cache))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logins_are_cleaned_and_checked() {
        assert_eq!(clean_login(" @juliohm ").as_deref(), Some("juliohm"));
        assert_eq!(clean_login("Julio-HM").as_deref(), Some("Julio-HM"));
        assert!(clean_login("").is_none());
        assert!(clean_login("@").is_none());
        assert!(clean_login("x is:open").is_none());
        assert!(clean_login("-lead").is_none());
        assert!(clean_login("trail-").is_none());
        assert!(clean_login(&"a".repeat(40)).is_none());
    }
}
