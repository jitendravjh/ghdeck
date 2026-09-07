use crate::model::*;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::time::Duration;

const API: &str = "https://api.github.com/graphql";
const REST: &str = "https://api.github.com/notifications";
const UA: &str = concat!("ghwork/", env!("CARGO_PKG_VERSION"));
const RETRIES: u32 = 3;

fn retryable(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::StatusCode(c) => *c >= 500 || *c == 429,
        ureq::Error::Timeout(_) | ureq::Error::Io(_) | ureq::Error::ConnectionFailed => true,
        _ => false,
    }
}

const QUERY: &str = r#"
query($involves:String!,$reviews:String!,$n:Int!,$c1:String,$c2:String){
  involves: search(query:$involves,type:ISSUE,first:$n,after:$c1){
    issueCount pageInfo{hasNextPage endCursor} nodes{...Row} }
  reviews: search(query:$reviews,type:ISSUE,first:$n,after:$c2){
    issueCount pageInfo{hasNextPage endCursor} nodes{...Row} }
  rateLimit{cost remaining}
}
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
    pub involved_total: u64,
    pub cost: u64,
    pub remaining: u64,
}

impl Client {
    pub fn new(token: String) -> Self {
        Self { token, agent: ureq::Agent::new_with_defaults() }
    }

    fn graphql(&self, vars: Value) -> Result<Value> {
        let body = json!({ "query": QUERY, "variables": vars });
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

            let v: Value = res.body_mut().read_json().context("bad graphql response")?;
            if let Some(errs) = v.get("errors") {
                bail!("github returned errors: {errs}");
            }
            return v.get("data").cloned().context("graphql response had no data");
        }
        Err(last.unwrap_or_else(|| anyhow::anyhow!("github graphql request failed")))
    }

    pub fn fetch(&self, pages: usize, per_page: u64) -> Result<Fetched> {
        let mut items: Vec<Item> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let (mut c1, mut c2) = (Value::Null, Value::Null);
        let (mut cost, mut remaining, mut total) = (0u64, 0u64, 0u64);

        for _ in 0..pages {
            let data = self.graphql(json!({
                "involves": "involves:@me sort:updated-desc",
                "reviews": "is:pr review-requested:@me sort:updated-desc",
                "n": per_page,
                "c1": c1,
                "c2": c2,
            }))?;

            cost += data["rateLimit"]["cost"].as_u64().unwrap_or(0);
            remaining = data["rateLimit"]["remaining"].as_u64().unwrap_or(remaining);
            if total == 0 {
                total = data["involves"]["issueCount"].as_u64().unwrap_or(0);
            }

            for bucket in ["involves", "reviews"] {
                let requested = bucket == "reviews";
                for node in data[bucket]["nodes"].as_array().into_iter().flatten() {
                    if node.is_null() {
                        continue;
                    }
                    if let Some(mut it) = parse(node) {
                        it.review_requested_of_me = requested;
                        if seen.insert(it.url.clone()) {
                            items.push(it);
                        } else if requested {
                            if let Some(e) = items.iter_mut().find(|e| e.url == it.url) {
                                e.review_requested_of_me = true;
                            }
                        }
                    }
                }
            }

            c1 = next_cursor(&data["involves"]["pageInfo"]);
            c2 = next_cursor(&data["reviews"]["pageInfo"]);
            if c1.is_null() && c2.is_null() {
                break;
            }
        }

        items.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
        Ok(Fetched { items, involved_total: total, cost, remaining })
    }

    pub fn notifications_changed(&self, last_modified: Option<&str>) -> Result<(bool, Option<String>)> {
        let mut req = self
            .agent
            .get(REST)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", UA);
        if let Some(lm) = last_modified {
            req = req.header("If-Modified-Since", lm);
        }
        let res = req.call().context("notifications poll failed")?;
        let lm = res
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .or_else(|| last_modified.map(String::from));
        Ok((res.status() != 304, lm))
    }
}

fn next_cursor(page_info: &Value) -> Value {
    if page_info["hasNextPage"].as_bool().unwrap_or(false) {
        page_info["endCursor"].clone()
    } else {
        Value::Null
    }
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
        state_reason: n["stateReason"].as_str().map(String::from),
    })
}
