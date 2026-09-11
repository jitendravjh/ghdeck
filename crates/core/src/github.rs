use crate::model::*;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::time::Duration;

const API: &str = "https://api.github.com/graphql";
const REST: &str = "https://api.github.com/notifications";
const HOST: &str = "https://api.github.com";
const UA: &str = concat!("ghdeck/", env!("CARGO_PKG_VERSION"));
const RETRIES: u32 = 3;

fn retryable(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::StatusCode(c) => *c >= 500 || *c == 429,
        ureq::Error::Timeout(_) | ureq::Error::Io(_) | ureq::Error::ConnectionFailed => true,
        _ => false,
    }
}

const SEARCH: &str = r#"
query($main:String!,$extra:String!,$n:Int!,$c1:String,$c2:String){
  viewer{login}
  main: search(query:$main,type:ISSUE,first:$n,after:$c1){
    issueCount pageInfo{hasNextPage endCursor} nodes{...Row} }
  extra: search(query:$extra,type:ISSUE,first:$n,after:$c2){
    issueCount pageInfo{hasNextPage endCursor} nodes{...Row} }
  rateLimit{cost remaining}
}
"#;

const ROW: &str = r#"
fragment Row on SearchResultItem{
  ... on Issue{ __typename number title url state stateReason updatedAt
    repository{nameWithOwner} author{login}
    assignees(first:5){nodes{login}} labels(first:8){nodes{name}}
    comments(last:1){totalCount nodes{author{login} createdAt}} }
  ... on PullRequest{ __typename number title url state isDraft updatedAt
    repository{nameWithOwner} author{login}
    merged mergeable additions deletions changedFiles reviewDecision
    assignees(first:5){nodes{login}} labels(first:8){nodes{name}}
    comments(last:1){totalCount nodes{author{login} createdAt}}
    reviews(last:10){totalCount nodes{state author{login} submittedAt}}
    reviewRequests(first:8){nodes{requestedReviewer{... on User{login} ... on Team{name}}}}
    commits(last:1){nodes{commit{statusCheckRollup{state}}}} }
}
"#;

pub struct Client {
    token: String,
    agent: ureq::Agent,
}

pub struct Fetched {
    pub items: Vec<Item>,
    pub login: String,
    pub involved_total: u64,
    pub cost: u64,
    pub remaining: u64,
    pub feed_since: Option<DateTime<Utc>>,
}

impl Client {
    pub fn new(token: String) -> Self {
        Self { token, agent: ureq::Agent::new_with_defaults() }
    }

    fn graphql(&self, query: &str, vars: Value) -> Result<Value> {
        let v = self.post(query, vars)?;
        if let Some(errs) = v.get("errors") {
            bail!("github returned errors: {errs}");
        }
        v.get("data").cloned().context("graphql response had no data")
    }

    // keeps what came back when only some fields failed, like a repo deleted since the event
    fn graphql_partial(&self, query: &str, vars: Value) -> Result<Value> {
        let v = self.post(query, vars)?;
        match v.get("data") {
            Some(d) if !d.is_null() => Ok(d.clone()),
            _ => bail!("github returned errors: {}", v.get("errors").cloned().unwrap_or_default()),
        }
    }

    fn post(&self, query: &str, vars: Value) -> Result<Value> {
        let body = json!({ "query": query, "variables": vars });
        let mut last: Option<anyhow::Error> = None;

        for attempt in 0..RETRIES {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(400 << attempt));
            }
            let sent = self
                .agent
                .post(API)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", UA)
                .send_json(&body);

            let mut res = match sent {
                Ok(r) => r,
                Err(e) if retryable(&e) => {
                    last = Some(anyhow::Error::new(e).context("github was unreachable"));
                    continue;
                }
                Err(e) => return Err(e).context("github graphql request failed"),
            };

            return res.body_mut().read_json().context("bad graphql response");
        }
        Err(last.unwrap_or_else(|| anyhow::anyhow!("github graphql request failed")))
    }

    pub fn fetch(&self, pages: usize, per_page: u64) -> Result<Fetched> {
        self.collect(
            "involves:@me sort:updated-desc",
            "is:pr review-requested:@me sort:updated-desc",
            |it| it.review_requested_of_me = true,
            pages,
            per_page,
            &mut |_, _| true,
        )
    }

    pub fn activity(
        &self,
        login: &str,
        pages: usize,
        per_page: u64,
        on_page: &mut dyn FnMut(&[Item], u64) -> bool,
    ) -> Result<Fetched> {
        let got = self.collect(
            &format!("involves:{login} sort:updated-desc"),
            &format!("mentions:{login} sort:updated-desc"),
            |it| it.mentioned = true,
            pages,
            per_page,
            on_page,
        )?;
        if !got.items.is_empty() {
            return Ok(got);
        }
        // graphql says zero both for no public work and for an account github keeps out of search,
        // the rest search says which, and the user lookup tells a hidden account from a missing one
        let (status, _) = self.rest(&format!("{HOST}/search/issues?q=author:{login}&per_page=1"))?;
        if status != 422 {
            return Ok(got);
        }
        let (status, _) = self.rest(&format!("{HOST}/users/{login}"))?;
        if status == 404 {
            bail!("there is no github user called {login}");
        }
        self.activity_feed(login, per_page, on_page)
    }

    fn activity_feed(
        &self,
        login: &str,
        per_page: u64,
        on_page: &mut dyn FnMut(&[Item], u64) -> bool,
    ) -> Result<Fetched> {
        // github keeps 300 events over three pages, and a page can come back short with more after it
        let mut events = Vec::new();
        for page in 1..=3 {
            let url = format!("{HOST}/users/{login}/events/public?per_page=100&page={page}");
            match self.rest(&url)? {
                (200, Value::Array(batch)) if !batch.is_empty() => events.extend(batch),
                _ => break,
            }
        }
        let (refs, since) = feed_refs(&events);

        let mut items: Vec<Item> = Vec::new();
        let (mut cost, mut remaining) = (0u64, 0u64);
        for chunk in refs.chunks(per_page.max(1) as usize) {
            let data = self.by_refs(chunk)?;
            cost += data["rateLimit"]["cost"].as_u64().unwrap_or(0);
            remaining = data["rateLimit"]["remaining"].as_u64().unwrap_or(remaining);
            items.extend((0..chunk.len()).filter_map(|i| parse(&data[format!("r{i}")]["issueOrPullRequest"])));
            items.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
            if !on_page(&items, items.len() as u64) {
                break;
            }
        }
        Ok(Fetched { involved_total: items.len() as u64, items, login: String::new(), cost, remaining, feed_since: since })
    }

    fn by_refs(&self, refs: &[(String, u64)]) -> Result<Value> {
        let mut decl = Vec::new();
        let mut fields = String::new();
        let mut vars = serde_json::Map::new();
        for (i, (repo, number)) in refs.iter().enumerate() {
            let Some((owner, name)) = repo.split_once('/') else { continue };
            decl.push(format!("$o{i}:String!,$n{i}:String!,$k{i}:Int!"));
            fields.push_str(&format!("r{i}: repository(owner:$o{i},name:$n{i}){{issueOrPullRequest(number:$k{i}){{...Row}}}}\n"));
            vars.insert(format!("o{i}"), owner.into());
            vars.insert(format!("n{i}"), name.into());
            vars.insert(format!("k{i}"), (*number).into());
        }
        if decl.is_empty() {
            return Ok(Value::Null);
        }
        let query = format!("query({}){{\n{fields}rateLimit{{cost remaining}}\n}}\n{ROW}", decl.join(","));
        self.graphql_partial(&query, Value::Object(vars))
    }

    fn rest(&self, url: &str) -> Result<(u16, Value)> {
        let sent = self
            .agent
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", UA)
            .call();
        match sent {
            Ok(mut res) => {
                let status = res.status().as_u16();
                Ok((status, res.body_mut().read_json().unwrap_or_default()))
            }
            Err(ureq::Error::StatusCode(c)) => Ok((c, Value::Null)),
            Err(e) => Err(e).context("github was unreachable"),
        }
    }

    // on_page sees everything so far after each page, and stops the fetch by returning false
    fn collect(
        &self,
        main: &str,
        extra: &str,
        mark: fn(&mut Item),
        pages: usize,
        per_page: u64,
        on_page: &mut dyn FnMut(&[Item], u64) -> bool,
    ) -> Result<Fetched> {
        let mut items: Vec<Item> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let (mut c1, mut c2) = (Value::Null, Value::Null);
        let (mut cost, mut remaining, mut total) = (0u64, 0u64, 0u64);
        let mut login = String::new();
        let query = format!("{SEARCH}{ROW}");

        for _ in 0..pages {
            let data = self.graphql(&query, json!({
                "main": main,
                "extra": extra,
                "n": per_page,
                "c1": c1,
                "c2": c2,
            }))?;

            if login.is_empty() {
                login = data["viewer"]["login"].as_str().unwrap_or_default().to_string();
            }
            cost += data["rateLimit"]["cost"].as_u64().unwrap_or(0);
            remaining = data["rateLimit"]["remaining"].as_u64().unwrap_or(remaining);
            if total == 0 {
                total = data["main"]["issueCount"].as_u64().unwrap_or(0);
            }

            for bucket in ["main", "extra"] {
                let marked = bucket == "extra";
                for node in data[bucket]["nodes"].as_array().into_iter().flatten() {
                    if node.is_null() {
                        continue;
                    }
                    if let Some(mut it) = parse(node) {
                        if marked {
                            mark(&mut it);
                        }
                        if seen.insert(it.url.clone()) {
                            items.push(it);
                        } else if marked {
                            if let Some(e) = items.iter_mut().find(|e| e.url == it.url) {
                                mark(e);
                            }
                        }
                    }
                }
            }

            items.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
            if !on_page(&items, total) {
                break;
            }
            c1 = next_cursor(&data["main"]["pageInfo"]);
            c2 = next_cursor(&data["extra"]["pageInfo"]);
            if c1.is_null() && c2.is_null() {
                break;
            }
        }

        Ok(Fetched { items, login, involved_total: total, cost, remaining, feed_since: None })
    }

    pub fn thread(&self, repo: &str, number: u64) -> Result<Vec<crate::thread::Event>> {
        let (owner, name) = repo.split_once('/').context("repo was not owner/name")?;
        let data = self.graphql(
            crate::thread::QUERY,
            json!({ "owner": owner, "name": name, "number": number }),
        )?;
        Ok(crate::thread::parse(&data))
    }

    pub fn notifications_changed(&self, last_modified: Option<&str>) -> Result<Poll> {
        let mut req = self
            .agent
            .get(REST)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", UA);
        if let Some(lm) = last_modified {
            req = req.header("If-Modified-Since", lm);
        }
        let res = req.call().context("notifications poll failed")?;
        let header = |name: &str| {
            res.headers().get(name).and_then(|v| v.to_str().ok()).map(String::from)
        };
        Ok(Poll {
            changed: res.status() != 304,
            last_modified: header("last-modified").or_else(|| last_modified.map(String::from)),
            interval: header("x-poll-interval").and_then(|v| v.parse().ok()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Poll {
    pub changed: bool,
    pub last_modified: Option<String>,
    pub interval: Option<u64>,
}

fn next_cursor(page_info: &Value) -> Value {
    if page_info["hasNextPage"].as_bool().unwrap_or(false) {
        page_info["endCursor"].clone()
    } else {
        Value::Null
    }
}

// the prs and issues someone touched, once each and newest first, plus how far back the feed goes
fn feed_refs(events: &[Value]) -> (Vec<(String, u64)>, Option<DateTime<Utc>>) {
    let mut seen = std::collections::HashSet::new();
    let mut refs = Vec::new();
    let mut oldest: Option<DateTime<Utc>> = None;
    for e in events {
        if let Some(t) = e["created_at"].as_str().and_then(|s| s.parse::<DateTime<Utc>>().ok()) {
            oldest = Some(oldest.map_or(t, |o| o.min(t)));
        }
        let key = match e["type"].as_str() {
            Some("PullRequestEvent" | "PullRequestReviewEvent" | "PullRequestReviewCommentEvent") => "pull_request",
            Some("IssuesEvent" | "IssueCommentEvent") => "issue",
            _ => continue,
        };
        let (Some(repo), Some(number)) = (e["repo"]["name"].as_str(), e["payload"][key]["number"].as_u64()) else {
            continue;
        };
        if seen.insert((repo.to_string(), number)) {
            refs.push((repo.to_string(), number));
        }
    }
    (refs, oldest)
}

fn names(v: &Value, path: &str) -> Vec<String> {
    v[path]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["login"].as_str().or_else(|| n["name"].as_str()))
        .map(String::from)
        .collect()
}

fn parse(n: &Value) -> Option<Item> {
    let kind = match n["__typename"].as_str()? {
        "PullRequest" => Kind::Pr,
        "Issue" => Kind::Issue,
        _ => return None,
    };
    let merged = n["merged"].as_bool().unwrap_or(false);
    let draft = n["isDraft"].as_bool().unwrap_or(false);
    let closed = n["state"].as_str() == Some("CLOSED");
    let state = if merged {
        State::Merged
    } else if closed {
        State::Closed
    } else if draft {
        State::Draft
    } else {
        State::Open
    };

    let mergeable = n["mergeable"].as_str();
    let ci = match n["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"].as_str() {
        Some("SUCCESS") => Ci::Success,
        Some("FAILURE") => Ci::Failure,
        Some("PENDING") => Ci::Pending,
        Some("ERROR") => Ci::Error,
        _ => Ci::None,
    };
    let review = match n["reviewDecision"].as_str() {
        Some("APPROVED") => Some(Review::Approved),
        Some("CHANGES_REQUESTED") => Some(Review::ChangesRequested),
        Some("REVIEW_REQUIRED") => Some(Review::ReviewRequired),
        _ => None,
    };

    let mut last: Option<(DateTime<Utc>, String, String)> = None;
    let mut consider = |ts: Option<&str>, who: Option<&str>, what: &str| {
        if let (Some(ts), Some(who)) = (ts, who) {
            if let Ok(t) = ts.parse::<DateTime<Utc>>() {
                if last.as_ref().is_none_or(|(p, _, _)| t > *p) {
                    last = Some((t, who.to_string(), what.to_string()));
                }
            }
        }
    };
    for r in n["reviews"]["nodes"].as_array().into_iter().flatten() {
        let what = match r["state"].as_str() {
            Some("APPROVED") => "approved",
            Some("CHANGES_REQUESTED") => "requested changes",
            Some("DISMISSED") => "review dismissed",
            _ => "reviewed",
        };
        consider(r["submittedAt"].as_str(), r["author"]["login"].as_str(), what);
    }
    for c in n["comments"]["nodes"].as_array().into_iter().flatten() {
        consider(c["createdAt"].as_str(), c["author"]["login"].as_str(), "commented");
    }

    let waiting_on = n["reviewRequests"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            r["requestedReviewer"]["login"]
                .as_str()
                .or_else(|| r["requestedReviewer"]["name"].as_str())
        })
        .map(String::from)
        .collect();

    Some(Item {
        url: n["url"].as_str()?.to_string(),
        repo: n["repository"]["nameWithOwner"].as_str()?.to_string(),
        number: n["number"].as_u64()?,
        title: n["title"].as_str().unwrap_or_default().trim().to_string(),
        kind,
        state,
        updated_at: n["updatedAt"].as_str()?.parse().ok()?,
        author: n["author"]["login"].as_str().unwrap_or("ghost").to_string(),
        conflicting: mergeable == Some("CONFLICTING"),
        mergeable_unknown: mergeable == Some("UNKNOWN"),
        review,
        ci,
        comments: n["comments"]["totalCount"].as_u64().unwrap_or(0),
        reviews_left: n["reviews"]["totalCount"].as_u64().unwrap_or(0),
        waiting_on,
        assignees: names(n, "assignees"),
        labels: names(n, "labels"),
        additions: n["additions"].as_u64().unwrap_or(0),
        deletions: n["deletions"].as_u64().unwrap_or(0),
        changed_files: n["changedFiles"].as_u64().unwrap_or(0),
        last_actor: last.as_ref().map(|(_, w, _)| w.clone()),
        last_action: last.as_ref().map(|(_, _, a)| a.clone()),
        review_requested_of_me: false,
        mentioned: false,
        state_reason: n["stateReason"].as_str().map(String::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(extra: &str) -> Value {
        serde_json::from_str(extra).unwrap()
    }

    #[test]
    fn parses_a_conflicting_pr_with_failing_ci() {
        let it = parse(&node(
            r#"{
              "__typename":"PullRequest","number":134,"title":" implement CNO ",
              "url":"https://github.com/o/r/pull/134","state":"OPEN","isDraft":false,
              "updatedAt":"2026-05-19T04:32:08Z","repository":{"nameWithOwner":"o/r"},
              "author":{"login":"jitendravjh"},"merged":false,"mergeable":"CONFLICTING",
              "additions":222,"deletions":2,"changedFiles":4,"reviewDecision":null,
              "comments":{"totalCount":2,"nodes":[{"author":{"login":"bob"},"createdAt":"2026-05-18T00:00:00Z"}]},
              "reviews":{"totalCount":0,"nodes":[]},
              "reviewRequests":{"nodes":[]},
              "labels":{"nodes":[{"name":"enhancement"}]},
              "assignees":{"nodes":[]},
              "commits":{"nodes":[{"commit":{"statusCheckRollup":{"state":"FAILURE"}}}]}
            }"#,
        ))
        .expect("should parse");

        assert_eq!(it.kind, Kind::Pr);
        assert_eq!(it.state, State::Open);
        assert_eq!(it.title, "implement CNO");
        assert!(it.conflicting);
        assert_eq!(it.ci, Ci::Failure);
        assert_eq!(it.labels, vec!["enhancement"]);
        assert_eq!(it.last_actor.as_deref(), Some("bob"));
        assert_eq!(it.last_action.as_deref(), Some("commented"));
        assert!(it.needs_attention());
    }

    #[test]
    fn newest_event_wins_between_reviews_and_comments() {
        let it = parse(&node(
            r#"{
              "__typename":"PullRequest","number":1,"title":"t",
              "url":"u","state":"OPEN","updatedAt":"2026-01-03T00:00:00Z",
              "repository":{"nameWithOwner":"o/r"},"author":{"login":"a"},
              "merged":false,"mergeable":"MERGEABLE","reviewDecision":"CHANGES_REQUESTED",
              "comments":{"totalCount":1,"nodes":[{"author":{"login":"early"},"createdAt":"2026-01-01T00:00:00Z"}]},
              "reviews":{"totalCount":1,"nodes":[{"state":"CHANGES_REQUESTED","author":{"login":"late"},"submittedAt":"2026-01-02T00:00:00Z"}]},
              "reviewRequests":{"nodes":[{"requestedReviewer":{"login":"carol"}},{"requestedReviewer":{"name":"team-x"}}]},
              "labels":{"nodes":[]},"assignees":{"nodes":[]},
              "commits":{"nodes":[]}
            }"#,
        ))
        .unwrap();

        assert_eq!(it.last_actor.as_deref(), Some("late"));
        assert_eq!(it.last_action.as_deref(), Some("requested changes"));
        assert_eq!(it.waiting_on, vec!["carol", "team-x"]);
        assert_eq!(it.ci, Ci::None);
    }

    #[test]
    fn merged_beats_closed_and_draft() {
        let it = parse(&node(
            r#"{
              "__typename":"PullRequest","number":2,"title":"t","url":"u",
              "state":"CLOSED","isDraft":true,"merged":true,
              "updatedAt":"2026-01-01T00:00:00Z","repository":{"nameWithOwner":"o/r"},
              "author":{"login":"a"},"comments":{"totalCount":0,"nodes":[]},
              "reviews":{"totalCount":0,"nodes":[]},"reviewRequests":{"nodes":[]},
              "labels":{"nodes":[]},"assignees":{"nodes":[]},"commits":{"nodes":[]}
            }"#,
        ))
        .unwrap();
        assert_eq!(it.state, State::Merged);
        assert!(!it.is_open());
        assert!(!it.needs_attention());
    }

    #[test]
    fn issue_keeps_state_reason_and_has_no_pr_fields() {
        let it = parse(&node(
            r#"{
              "__typename":"Issue","number":9,"title":"broken","url":"u",
              "state":"CLOSED","stateReason":"NOT_PLANNED",
              "updatedAt":"2026-02-01T00:00:00Z","repository":{"nameWithOwner":"o/r"},
              "author":{"login":"a"},"comments":{"totalCount":3,"nodes":[]},
              "labels":{"nodes":[]},"assignees":{"nodes":[{"login":"me"}]}
            }"#,
        ))
        .unwrap();
        assert_eq!(it.kind, Kind::Issue);
        assert_eq!(it.state_reason.as_deref(), Some("NOT_PLANNED"));
        assert_eq!(it.assignees, vec!["me"]);
        assert_eq!(it.ci, Ci::None);
        assert_eq!(it.additions, 0);
        assert!(it.chips().iter().any(|(t, _)| t.ends_with("not planned")));
    }

    #[test]
    fn closed_items_drop_stale_review_and_ci_chips() {
        let it = parse(&node(
            r#"{
              "__typename":"PullRequest","number":3,"title":"t","url":"u",
              "state":"CLOSED","merged":false,"mergeable":"CONFLICTING",
              "reviewDecision":"REVIEW_REQUIRED",
              "updatedAt":"2026-01-01T00:00:00Z","repository":{"nameWithOwner":"o/r"},
              "author":{"login":"a"},"comments":{"totalCount":1,"nodes":[]},
              "reviews":{"totalCount":0,"nodes":[]},"reviewRequests":{"nodes":[]},
              "labels":{"nodes":[]},"assignees":{"nodes":[]},
              "commits":{"nodes":[{"commit":{"statusCheckRollup":{"state":"SUCCESS"}}}]}
            }"#,
        ))
        .unwrap();
        let chips: Vec<String> = it.chips().into_iter().map(|(t, _)| t).collect();
        assert_eq!(chips, vec!["\u{2716} closed".to_string()]);
    }

    #[test]
    fn unknown_typename_is_skipped() {
        assert!(parse(&node(r#"{"__typename":"Repository"}"#)).is_none());
        assert!(parse(&node(r#"{}"#)).is_none());
    }

    #[test]
    fn feed_keeps_each_pr_and_issue_once_newest_first() {
        let events: Vec<Value> = serde_json::from_str(
            r#"[
              {"type":"PullRequestEvent","created_at":"2026-09-10T00:00:00Z","repo":{"name":"o/a"},"payload":{"pull_request":{"number":7}}},
              {"type":"PushEvent","created_at":"2026-09-09T00:00:00Z","repo":{"name":"o/a"},"payload":{}},
              {"type":"IssueCommentEvent","created_at":"2026-09-08T00:00:00Z","repo":{"name":"o/b"},"payload":{"issue":{"number":3}}},
              {"type":"PullRequestReviewEvent","created_at":"2026-09-07T00:00:00Z","repo":{"name":"o/a"},"payload":{"pull_request":{"number":7}}},
              {"type":"IssuesEvent","created_at":"2026-09-01T00:00:00Z","repo":{"name":"o/c"},"payload":{"issue":{"number":1}}}
            ]"#,
        )
        .unwrap();
        let (refs, since) = feed_refs(&events);
        assert_eq!(refs, vec![("o/a".to_string(), 7), ("o/b".to_string(), 3), ("o/c".to_string(), 1)]);
        assert_eq!(since.unwrap().to_rfc3339(), "2026-09-01T00:00:00+00:00");
    }
}
