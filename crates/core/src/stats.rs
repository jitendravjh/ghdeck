//! The figures behind the stats screen. Pure arithmetic over items the app
//! already holds, so it can be checked without touching the network.

use crate::model::{Item, Kind, State};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::{BTreeMap, HashSet};

/// The five counts across the top.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    pub involved: usize,
    pub prs: usize,
    pub merged: usize,
    pub issues: usize,
    pub open_issues: usize,
    pub reviews: usize,
    pub discussions: usize,
}

/// What a person did, as opposed to what they merely touched.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Work {
    pub prs_opened: usize,
    pub prs_merged: usize,
    pub issues_opened: usize,
    pub issues_closed: usize,
    pub repos: usize,
    pub active_days: usize,
    pub longest_streak: usize,
    pub current_streak: usize,
    pub median_merge_hours: Option<i64>,
    pub busiest: Option<(NaiveDate, usize)>,
}

pub fn counts(items: &[&Item], me: &str) -> Counts {
    let mine = |it: &Item| it.author.eq_ignore_ascii_case(me);
    let prs: Vec<&&Item> = items.iter().filter(|it| it.kind == Kind::Pr).collect();
    let issues: Vec<&&Item> = items.iter().filter(|it| it.kind == Kind::Issue).collect();
    Counts {
        involved: items.len(),
        prs: prs.len(),
        merged: prs.iter().filter(|it| it.state == State::Merged).count(),
        issues: issues.len(),
        open_issues: issues.iter().filter(|it| it.is_open()).count(),
        reviews: prs.iter().filter(|it| !mine(it)).count(),
        discussions: issues.iter().filter(|it| !mine(it)).count(),
    }
}

pub fn work(items: &[&Item], me: &str, now: DateTime<Utc>) -> Work {
    let mine: Vec<&&Item> = items
        .iter()
        .filter(|it| it.author.eq_ignore_ascii_case(me))
        .collect();
    let prs: Vec<&&&Item> = mine.iter().filter(|it| it.kind == Kind::Pr).collect();
    let issues: Vec<&&&Item> = mine.iter().filter(|it| it.kind == Kind::Issue).collect();

    let mut per_day: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    for it in &mine {
        let when = it.created_at.unwrap_or(it.updated_at).date_naive();
        *per_day.entry(when).or_default() += 1;
    }

    let days: Vec<NaiveDate> = per_day.keys().copied().collect();
    let mut longest = 0usize;
    let mut run = 0usize;
    for (i, d) in days.iter().enumerate() {
        run = if i > 0 && *d == days[i - 1] + Duration::days(1) {
            run + 1
        } else {
            1
        };
        longest = longest.max(run);
    }

    let today = now.date_naive();
    let mut current = 0usize;
    let mut cursor = if per_day.contains_key(&today) {
        today
    } else {
        today - Duration::days(1)
    };
    while per_day.contains_key(&cursor) {
        current += 1;
        cursor -= Duration::days(1);
    }

    let mut waits: Vec<i64> = prs
        .iter()
        .filter_map(|it| match (it.created_at, it.merged_at) {
            (Some(a), Some(b)) if b >= a => Some((b - a).num_hours()),
            _ => None,
        })
        .collect();
    waits.sort_unstable();
    let median = if waits.is_empty() {
        None
    } else if waits.len() % 2 == 1 {
        Some(waits[waits.len() / 2])
    } else {
        Some((waits[waits.len() / 2 - 1] + waits[waits.len() / 2]) / 2)
    };

    let busiest = per_day
        .iter()
        .max_by_key(|(d, n)| (**n, **d))
        .map(|(d, n)| (*d, *n));
    let repos: HashSet<String> = items.iter().map(|it| it.repo.to_lowercase()).collect();

    Work {
        prs_opened: prs.len(),
        prs_merged: prs.iter().filter(|it| it.state == State::Merged).count(),
        issues_opened: issues.len(),
        issues_closed: issues.iter().filter(|it| !it.is_open()).count(),
        repos: repos.len(),
        active_days: per_day.len(),
        longest_streak: longest,
        current_streak: current,
        median_merge_hours: median,
        busiest,
    }
}

pub fn percent(part: usize, whole: usize) -> usize {
    (part * 100 + whole / 2).checked_div(whole).unwrap_or(0)
}

pub fn spell_hours(h: i64) -> String {
    if h < 1 {
        "under an hour".into()
    } else if h < 48 {
        format!("{h} h")
    } else {
        format!("{} d", h / 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ci;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap()
    }

    fn item(author: &str, kind: Kind, state: State, opened_days_ago: i64, repo: &str) -> Item {
        let created = now() - Duration::days(opened_days_ago);
        Item {
            url: format!("{repo}-{author}-{opened_days_ago}-{:?}", kind),
            repo: repo.into(),
            number: 1,
            title: "t".into(),
            kind,
            state,
            updated_at: created,
            created_at: Some(created),
            merged_at: if state == State::Merged {
                Some(created + Duration::hours(10))
            } else {
                None
            },
            author: author.into(),
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
    fn counts_split_mine_from_everyone_elses() {
        let a = item("me", Kind::Pr, State::Merged, 1, "o/a");
        let b = item("you", Kind::Pr, State::Open, 2, "o/a");
        let c = item("me", Kind::Issue, State::Open, 3, "o/b");
        let d = item("you", Kind::Issue, State::Closed, 4, "o/b");
        let all = vec![&a, &b, &c, &d];
        let c2 = counts(&all, "me");
        assert_eq!(c2.involved, 4);
        assert_eq!(c2.prs, 2);
        assert_eq!(c2.merged, 1);
        assert_eq!(c2.issues, 2);
        assert_eq!(c2.open_issues, 1);
        assert_eq!(c2.reviews, 1, "a pull request somebody else opened");
        assert_eq!(c2.discussions, 1, "an issue somebody else opened");
    }

    #[test]
    fn a_login_in_a_different_case_is_still_me() {
        let a = item("Me", Kind::Pr, State::Open, 1, "o/a");
        assert_eq!(counts(&[&a], "me").reviews, 0);
    }

    #[test]
    fn streaks_count_consecutive_days_only() {
        let a = item("me", Kind::Pr, State::Open, 0, "o/a");
        let b = item("me", Kind::Pr, State::Open, 1, "o/a");
        let c = item("me", Kind::Pr, State::Open, 2, "o/a");
        let far = item("me", Kind::Pr, State::Open, 40, "o/a");
        let w = work(&[&a, &b, &c, &far], "me", now());
        assert_eq!(w.active_days, 4);
        assert_eq!(w.longest_streak, 3);
        assert_eq!(w.current_streak, 3, "today, yesterday, the day before");
    }

    #[test]
    fn a_gap_since_yesterday_ends_the_current_streak() {
        let old = item("me", Kind::Pr, State::Open, 5, "o/a");
        let w = work(&[&old], "me", now());
        assert_eq!(w.current_streak, 0);
        assert_eq!(w.longest_streak, 1);
    }

    #[test]
    fn the_median_merge_time_is_the_middle_one() {
        let mut a = item("me", Kind::Pr, State::Merged, 1, "o/a");
        let mut b = item("me", Kind::Pr, State::Merged, 2, "o/a");
        let mut c = item("me", Kind::Pr, State::Merged, 3, "o/a");
        a.merged_at = a.created_at.map(|t| t + Duration::hours(2));
        b.merged_at = b.created_at.map(|t| t + Duration::hours(6));
        c.merged_at = c.created_at.map(|t| t + Duration::hours(100));
        let w = work(&[&a, &b, &c], "me", now());
        assert_eq!(w.median_merge_hours, Some(6));
    }

    #[test]
    fn a_pull_request_never_merged_is_left_out_of_the_median() {
        let a = item("me", Kind::Pr, State::Open, 1, "o/a");
        assert_eq!(work(&[&a], "me", now()).median_merge_hours, None);
    }

    #[test]
    fn percentages_round_to_the_nearest_whole() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(0, 0), 0, "never divides by zero");
    }

    #[test]
    fn hours_read_as_hours_until_they_are_days() {
        assert_eq!(spell_hours(0), "under an hour");
        assert_eq!(spell_hours(19), "19 h");
        assert_eq!(spell_hours(72), "3 d");
    }
}
