pub mod auth;
pub mod cache;
pub mod github;
pub mod model;
pub mod stats;
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

/// Pull requests, issues, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    #[default]
    Any,
    Prs,
    Issues,
}

impl Shape {
    pub const ORDER: [Shape; 3] = [Shape::Any, Shape::Prs, Shape::Issues];

    pub fn label(self) -> &'static str {
        match self {
            Shape::Any => "PR+ISSUE",
            Shape::Prs => "PR",
            Shape::Issues => "ISSUE",
        }
    }

    pub fn keeps(self, it: &Item) -> bool {
        match self {
            Shape::Any => true,
            Shape::Prs => it.kind == Kind::Pr,
            Shape::Issues => it.kind == Kind::Issue,
        }
    }

    pub fn next(self, forward: bool) -> Shape {
        step(&Shape::ORDER, self, forward)
    }
}

fn step<T: Copy + PartialEq>(all: &[T], now: T, forward: bool) -> T {
    let at = all.iter().position(|x| *x == now).unwrap_or(0);
    let n = all.len();
    let to = if forward { (at + 1) % n } else { (at + n - 1) % n };
    all[to]
}

/// Everything the list is narrowed by, on top of the named filter. Two knobs,
/// each answering something the tabs cannot: what kind, and which repo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lens {
    pub shape: Shape,
    pub repo: Option<String>,
}

impl Lens {
    pub fn plain(&self) -> bool {
        *self == Lens::default()
    }

    pub fn keeps(&self, it: &Item) -> bool {
        self.shape.keeps(it)
            && match &self.repo {
                Some(repo) => it.repo.eq_ignore_ascii_case(repo),
                None => true,
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

#[cfg(test)]
mod lens_tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn at(kind: Kind, repo: &str) -> Item {
        Item {
            url: format!("{repo}/{kind:?}"),
            repo: repo.into(),
            number: 1,
            title: "t".into(),
            kind,
            state: State::Open,
            updated_at: Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap(),
            created_at: None,
            merged_at: None,
            author: "me".into(),
            conflicting: false,
            mergeable_unknown: false,
            review: None,
            ci: Ci::None,
            comments: 0,
            reviews_left: 0,
            waiting_on: vec![],
            assignees: vec![],
            labels: vec![],
            additions: 0,
            deletions: 0,
            changed_files: 0,
            last_actor: None,
            last_action: None,
            review_requested_of_me: false,
            mentioned: false,
            state_reason: None,
        }
    }

    #[test]
    fn a_fresh_lens_keeps_everything() {
        let lens = Lens::default();
        assert!(lens.plain());
        assert!(lens.keeps(&at(Kind::Pr, "o/r")));
        assert!(lens.keeps(&at(Kind::Issue, "o/r")));
    }

    #[test]
    fn the_kind_knob_picks_one_side() {
        let lens = Lens { shape: Shape::Issues, ..Lens::default() };
        assert!(!lens.keeps(&at(Kind::Pr, "o/r")));
        assert!(lens.keeps(&at(Kind::Issue, "o/r")));
    }

    #[test]
    fn a_repo_filter_ignores_case() {
        let lens = Lens { repo: Some("Owner/Repo".into()), ..Lens::default() };
        assert!(lens.keeps(&at(Kind::Pr, "owner/repo")));
        assert!(!lens.keeps(&at(Kind::Pr, "other/repo")));
    }

    #[test]
    fn cycling_wraps_round_both_ways() {
        assert_eq!(Shape::Any.next(true), Shape::Prs);
        assert_eq!(Shape::Any.next(false), Shape::Issues);
        assert_eq!(Shape::Issues.next(true), Shape::Any);
    }
}
