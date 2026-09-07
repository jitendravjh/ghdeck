use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub at: DateTime<Utc>,
    pub who: String,
    pub label: String,
    pub body: String,
    pub bot: bool,
}

impl Event {
    pub fn gist(&self) -> String {
        self.body
            .lines()
            .map(|l| plain(l.trim()))
            .find(|l| l.chars().any(char::is_alphanumeric))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect()
    }
}

fn plain(line: &str) -> String {
    let mut cur = line.to_string();
    for _ in 0..4 {
        let next = unlink(&cur);
        if next == cur {
            break;
        }
        cur = next;
    }
    tidy(&cur)
}

fn matching(chars: &[char], from: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(from) {
        if *c == open {
            depth += 1;
        } else if *c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn unlink(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close) = matching(&chars, i, '[', ']') {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(end) = matching(&chars, close + 1, '(', ')') {
                        if out.ends_with('!') {
                            out.pop();
                        }
                        out.extend(&chars[i + 1..close]);
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn tidy(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.replace("**", "")
        .trim()
        .trim_start_matches(['#', '!', ':', '*', '|', ' '])
        .trim()
        .to_string()
}

pub const QUERY: &str = r#"
query($owner:String!,$name:String!,$number:Int!){
  repository(owner:$owner,name:$name){
    issueOrPullRequest(number:$number){
      ... on Issue{ body author{login __typename} createdAt
        comments(last:30){nodes{author{login __typename} createdAt body}} }
      ... on PullRequest{ body author{login __typename} createdAt
        comments(last:30){nodes{author{login __typename} createdAt body}}
        reviews(last:20){nodes{author{login __typename} submittedAt state body
          comments(first:20){nodes{path line body}}}} }
    }
  }
  rateLimit{cost remaining}
}
"#;

fn login(v: &Value) -> String {
    v["author"]["login"].as_str().unwrap_or("ghost").to_string()
}

fn is_bot(v: &Value) -> bool {
    if v["author"]["__typename"].as_str() == Some("Bot") {
        return true;
    }
    let name = v["author"]["login"].as_str().unwrap_or_default();
    name.ends_with("[bot]")
        || matches!(
            name,
            "codecov" | "codecov-commenter" | "CLAassistant" | "github-actions"
                | "kubernetes-prow" | "k8s-ci-robot" | "dependabot" | "sonarcloud"
                | "netlify" | "vercel" | "coderabbitai" | "changeset-bot"
        )
}

fn at(v: &Value, key: &str) -> Option<DateTime<Utc>> {
    v[key].as_str()?.parse().ok()
}

fn clean(body: &str) -> String {
    body.replace("\r\n", "\n").trim().to_string()
}

pub fn parse(data: &Value) -> Vec<Event> {
    let node = &data["repository"]["issueOrPullRequest"];
    let mut out = Vec::new();

    if let Some(when) = at(node, "createdAt") {
        let body = clean(node["body"].as_str().unwrap_or_default());
        out.push(Event {
            at: when,
            who: login(node),
            label: "opened".into(),
            body,
            bot: is_bot(node),
        });
    }

    for c in node["comments"]["nodes"].as_array().into_iter().flatten() {
        if let Some(when) = at(c, "createdAt") {
            out.push(Event {
                at: when,
                who: login(c),
                label: "commented".into(),
                body: clean(c["body"].as_str().unwrap_or_default()),
                bot: is_bot(c),
            });
        }
    }

    for r in node["reviews"]["nodes"].as_array().into_iter().flatten() {
        let Some(when) = at(r, "submittedAt") else { continue };
        let who = login(r);
        let label = match r["state"].as_str() {
            Some("APPROVED") => "approved",
            Some("CHANGES_REQUESTED") => "requested changes",
            Some("DISMISSED") => "review dismissed",
            _ => "reviewed",
        };
        let bot = is_bot(r);
        let body = clean(r["body"].as_str().unwrap_or_default());
        if !body.is_empty() || r["comments"]["nodes"].as_array().is_none_or(|c| c.is_empty()) {
            out.push(Event { at: when, who: who.clone(), label: label.into(), body, bot });
        }
        for c in r["comments"]["nodes"].as_array().into_iter().flatten() {
            let where_ = match (c["path"].as_str(), c["line"].as_u64()) {
                (Some(p), Some(l)) => format!("on {p}:{l}"),
                (Some(p), None) => format!("on {p}"),
                _ => "on a line".into(),
            };
            out.push(Event {
                at: when,
                who: who.clone(),
                label: where_,
                body: clean(c["body"].as_str().unwrap_or_default()),
                bot,
            });
        }
    }

    out.retain(|e| !e.body.is_empty() || e.label != "commented");
    out.sort_by_key(|e| e.at);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_sorted_conversation_from_a_pull_request() {
        let data: Value = serde_json::from_str(
            r#"{"repository":{"issueOrPullRequest":{
              "body":"  first line\r\nsecond  ","author":{"login":"me"},"createdAt":"2026-01-01T00:00:00Z",
              "comments":{"nodes":[
                {"author":{"login":"bob"},"createdAt":"2026-01-05T00:00:00Z","body":"looks good"},
                {"author":{"login":"eve"},"createdAt":"2026-01-02T00:00:00Z","body":"needs work"}]},
              "reviews":{"nodes":[
                {"author":{"login":"carol"},"submittedAt":"2026-01-03T00:00:00Z","state":"CHANGES_REQUESTED",
                 "body":"see notes","comments":{"nodes":[{"path":"src/a.rs","line":42,"body":"rename this"}]}}]}
            }}}"#,
        ).unwrap();

        let evs = parse(&data);
        let seq: Vec<(&str, &str)> = evs.iter().map(|e| (e.who.as_str(), e.label.as_str())).collect();
        assert_eq!(
            seq,
            vec![
                ("me", "opened"),
                ("eve", "commented"),
                ("carol", "requested changes"),
                ("carol", "on src/a.rs:42"),
                ("bob", "commented"),
            ]
        );
        assert_eq!(evs[0].body, "first line\nsecond");
    }

    #[test]
    fn a_bare_approval_still_shows_up() {
        let data: Value = serde_json::from_str(
            r#"{"repository":{"issueOrPullRequest":{
              "body":"","author":{"login":"me"},"createdAt":"2026-01-01T00:00:00Z",
              "comments":{"nodes":[]},
              "reviews":{"nodes":[{"author":{"login":"juliohm"},"submittedAt":"2026-01-02T00:00:00Z",
                "state":"APPROVED","body":"","comments":{"nodes":[]}}]}
            }}}"#,
        ).unwrap();
        let evs = parse(&data);
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[1].label, "approved");
        assert!(evs[1].body.is_empty());
    }

    #[test]
    fn bots_are_flagged_and_summarised() {
        let data: Value = serde_json::from_str(
            r#"{"repository":{"issueOrPullRequest":{
              "body":"hi","author":{"login":"me","__typename":"User"},"createdAt":"2026-01-01T00:00:00Z",
              "comments":{"nodes":[
                {"author":{"login":"codecov","__typename":"User"},"createdAt":"2026-01-02T00:00:00Z",
                 "body":"<details>\n## Codecov Report\nPatch coverage is 93%"},
                {"author":{"login":"renovate[bot]","__typename":"User"},"createdAt":"2026-01-03T00:00:00Z",
                 "body":"bumped things"}]},
              "reviews":{"nodes":[]}
            }}}"#,
        ).unwrap();
        let evs = parse(&data);
        assert!(!evs[0].bot);
        assert!(evs[1].bot, "codecov should be a bot");
        assert!(evs[2].bot, "[bot] suffix should count");
        assert_eq!(evs[1].gist(), "Codecov Report");
        assert_eq!(plain("## [Codecov](https://x.y/z?a=b) Report :x:"), "Codecov Report :x:");
        assert_eq!(plain("see [issue #12] for context"), "see [issue #12] for context");
        assert_eq!(tidy("<details><summary>hi</summary>text"), "hitext");
    }

    #[test]
    fn empty_payload_gives_nothing() {
        assert!(parse(&serde_json::json!({})).is_empty());
    }
}

#[cfg(test)]
mod plain_tests {
    use super::plain;

    #[test]
    fn strips_a_plain_link() {
        assert_eq!(
            plain("[CLA assistant check](https://cla-assistant.io/x?pullRequest=1) <br/>Thank you!"),
            "CLA assistant check Thank you!"
        );
    }

    #[test]
    fn strips_a_nested_image_link() {
        assert_eq!(
            plain("[![CLA assistant check](https://cla-assistant.io/pull/badge/not_signed)](https://cla-assistant.io/calcom/cal.com?pullRequest=29686) <br/>Thank you for your submission!"),
            "CLA assistant check Thank you for your submission!"
        );
    }

    #[test]
    fn drops_bold_markers() {
        assert_eq!(plain("Welcome to **Cal.diy**, @me!"), "Welcome to Cal.diy, @me!");
    }

    #[test]
    fn leaves_a_bare_bracket_alone() {
        assert_eq!(plain("see [issue #12] for context"), "see [issue #12] for context");
    }

    #[test]
    fn handles_parens_inside_a_url() {
        assert_eq!(plain("read [the docs](https://x.y/a_(b)_c) now"), "read the docs now");
    }
}
