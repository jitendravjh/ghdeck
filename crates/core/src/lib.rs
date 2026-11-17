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

/// How far back the list reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    All,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl Span {
    pub const ORDER: [Span; 6] =
        [Span::All, Span::Day, Span::Week, Span::Month, Span::Quarter, Span::Year];

    pub fn label(self) -> &'static str {
        match self {
            Span::All => "ALL TIME",
            Span::Day => "24 HOURS",
            Span::Week => "7 DAYS",
            Span::Month => "30 DAYS",
            Span::Quarter => "90 DAYS",
            Span::Year => "12 MONTHS",
        }
    }

    pub fn days(self) -> Option<i64> {
        match self {
            Span::All => None,
            Span::Day => Some(1),
            Span::Week => Some(7),
            Span::Month => Some(30),
            Span::Quarter => Some(90),
            Span::Year => Some(365),
        }
    }

    pub fn next(self, forward: bool) -> Span {
        step(&Span::ORDER, self, forward)
    }
}

/// Pull requests, issues, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
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

/// Where a thing stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Any,
    Open,
    Merged,
    Closed,
    Draft,
}

impl Stage {
    pub const ORDER: [Stage; 5] =
        [Stage::Any, Stage::Open, Stage::Merged, Stage::Closed, Stage::Draft];

    pub fn label(self) -> &'static str {
        match self {
            Stage::Any => "ANY STATE",
            Stage::Open => "OPEN",
            Stage::Merged => "MERGED",
            Stage::Closed => "CLOSED",
            Stage::Draft => "DRAFT",
        }
    }

    pub fn keeps(self, it: &Item) -> bool {
        match self {
            Stage::Any => true,
            Stage::Open => it.state == State::Open,
            Stage::Merged => it.state == State::Merged,
            Stage::Closed => it.state == State::Closed,
            Stage::Draft => it.state == State::Draft,
        }
    }

    pub fn next(self, forward: bool) -> Stage {
        step(&Stage::ORDER, self, forward)
    }
}

/// The order rows are listed in. The website offers none of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Updated,
    Opened,
    Comments,
    Repo,
    Title,
}

impl Order {
    pub const ORDER: [Order; 5] =
        [Order::Updated, Order::Opened, Order::Comments, Order::Repo, Order::Title];

    pub fn label(self) -> &'static str {
        match self {
            Order::Updated => "LAST UPDATED",
            Order::Opened => "NEWEST",
            Order::Comments => "MOST TALKED ABOUT",
            Order::Repo => "BY REPO",
            Order::Title => "BY TITLE",
        }
    }

    pub fn next(self, forward: bool) -> Order {
        step(&Order::ORDER, self, forward)
    }
}

fn step<T: Copy + PartialEq>(all: &[T], now: T, forward: bool) -> T {
    let at = all.iter().position(|x| *x == now).unwrap_or(0);
    let n = all.len();
    let to = if forward { (at + 1) % n } else { (at + n - 1) % n };
    all[to]
}

/// Everything the list is narrowed by, beyond the named filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lens {
    pub span: Span,
    pub shape: Shape,
    pub stage: Stage,
    pub order: Order,
    pub repo: Option<String>,
}

impl Default for Lens {
    fn default() -> Self {
        Self {
            span: Span::All,
            shape: Shape::Any,
            stage: Stage::Any,
            order: Order::Updated,
            repo: None,
        }
    }
}

impl Lens {
    pub fn plain(&self) -> bool {
        *self == Lens::default()
    }

    pub fn keeps(&self, it: &Item, now: chrono::DateTime<chrono::Utc>) -> bool {
        if !self.shape.keeps(it) || !self.stage.keeps(it) {
            return false;
        }
        if let Some(repo) = &self.repo {
            if !it.repo.eq_ignore_ascii_case(repo) {
                return false;
            }
        }
        match self.span.days() {
            None => true,
            Some(d) => (now - it.updated_at).num_days() < d,
        }
    }

    pub fn sort(&self, items: &mut [&Item]) {
        match self.order {
            Order::Updated => items.sort_by_key(|x| std::cmp::Reverse(x.updated_at)),
            Order::Opened => {
                items.sort_by_key(|x| std::cmp::Reverse(x.created_at.unwrap_or(x.updated_at)))
            }
            Order::Comments => items.sort_by(|a, b| {
                b.comments.cmp(&a.comments).then(b.updated_at.cmp(&a.updated_at))
            }),
            Order::Repo => items.sort_by(|a, b| {
                a.repo.to_lowercase().cmp(&b.repo.to_lowercase()).then(b.updated_at.cmp(&a.updated_at))
            }),
            Order::Title => items.sort_by(|a, b| {
                a.title.to_lowercase().cmp(&b.title.to_lowercase())
            }),
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
    use chrono::{Duration, TimeZone, Utc};

    fn at(days_ago: i64, kind: Kind, state: State, comments: u64, repo: &str) -> Item {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        Item {
            url: format!("u{days_ago}{repo}"),
            repo: repo.into(),
            number: 1,
            title: format!("item {days_ago}"),
            kind,
            state,
            updated_at: now - Duration::days(days_ago),
            created_at: Some(now - Duration::days(days_ago + 10)),
            merged_at: None,
            author: "me".into(),
            conflicting: false,
            mergeable_unknown: false,
            review: None,
            ci: Ci::None,
            comments,
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

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap()
    }

    #[test]
    fn a_fresh_lens_keeps_everything() {
        let lens = Lens::default();
        assert!(lens.plain());
        for days in [0, 5, 100, 900] {
            assert!(lens.keeps(&at(days, Kind::Pr, State::Open, 0, "o/r"), now()));
        }
    }

    #[test]
    fn the_span_cuts_at_the_right_day() {
        let lens = Lens { span: Span::Week, ..Lens::default() };
        assert!(lens.keeps(&at(6, Kind::Pr, State::Open, 0, "o/r"), now()));
        assert!(!lens.keeps(&at(7, Kind::Pr, State::Open, 0, "o/r"), now()));
    }

    #[test]
    fn shape_and_stage_narrow_on_their_own() {
        let lens = Lens { shape: Shape::Issues, ..Lens::default() };
        assert!(!lens.keeps(&at(1, Kind::Pr, State::Open, 0, "o/r"), now()));
        assert!(lens.keeps(&at(1, Kind::Issue, State::Open, 0, "o/r"), now()));

        let lens = Lens { stage: Stage::Merged, ..Lens::default() };
        assert!(!lens.keeps(&at(1, Kind::Pr, State::Open, 0, "o/r"), now()));
        assert!(lens.keeps(&at(1, Kind::Pr, State::Merged, 0, "o/r"), now()));
    }

    #[test]
    fn draft_is_its_own_state_not_an_open_one() {
        let mut lens = Lens { stage: Stage::Open, ..Lens::default() };
        assert!(!lens.keeps(&at(1, Kind::Pr, State::Draft, 0, "o/r"), now()));
        lens.stage = Stage::Draft;
        assert!(lens.keeps(&at(1, Kind::Pr, State::Draft, 0, "o/r"), now()));
    }

    #[test]
    fn a_repo_filter_ignores_case() {
        let lens = Lens { repo: Some("Owner/Repo".into()), ..Lens::default() };
        assert!(lens.keeps(&at(1, Kind::Pr, State::Open, 0, "owner/repo"), now()));
        assert!(!lens.keeps(&at(1, Kind::Pr, State::Open, 0, "other/repo"), now()));
    }

    #[test]
    fn every_order_puts_the_right_row_first() {
        let a = at(1, Kind::Pr, State::Open, 2, "b/one");
        let b = at(9, Kind::Pr, State::Open, 40, "a/two");
        let mut lens = Lens::default();

        let pick = |lens: &Lens, a: &Item, b: &Item| {
            let mut v = vec![a, b];
            lens.sort(&mut v);
            v[0].repo.clone()
        };

        lens.order = Order::Updated;
        assert_eq!(pick(&lens, &a, &b), "b/one", "the most recently touched");
        lens.order = Order::Comments;
        assert_eq!(pick(&lens, &a, &b), "a/two", "the busiest thread");
        lens.order = Order::Repo;
        assert_eq!(pick(&lens, &a, &b), "a/two", "alphabetical by repo");
        lens.order = Order::Opened;
        assert_eq!(pick(&lens, &a, &b), "b/one", "the newest opened");
    }

    #[test]
    fn cycling_wraps_round_both_ways() {
        assert_eq!(Span::All.next(true), Span::Day);
        assert_eq!(Span::All.next(false), Span::Year);
        assert_eq!(Span::Year.next(true), Span::All);
        assert_eq!(Shape::Any.next(false), Shape::Issues);
        assert_eq!(Order::Updated.next(false), Order::Title);
    }
}

