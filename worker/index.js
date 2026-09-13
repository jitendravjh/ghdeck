// Serves the built site, and answers /api/user with someone's PRs and issues.
// The token lives in the GHDECK_TOKEN secret, browsers never see it.

const API = "https://api.github.com";
const PER_PAGE = 20;
const FEED_ROWS = 40;
const CACHE_SECONDS = 300;

const ROW = `
fragment Row on SearchResultItem{
  ... on Issue{ __typename number title url state stateReason updatedAt
    repository{nameWithOwner} author{login}
    comments(last:1){totalCount nodes{author{login} createdAt}} }
  ... on PullRequest{ __typename number title url state isDraft updatedAt
    repository{nameWithOwner} author{login}
    merged mergeable reviewDecision
    comments(last:1){totalCount nodes{author{login} createdAt}}
    reviews(last:10){totalCount nodes{state author{login} submittedAt}}
    reviewRequests(first:8){nodes{requestedReviewer{... on User{login}}}}
    commits(last:1){nodes{commit{statusCheckRollup{state}}}} }
}`;

const SEARCH = `
query($main:String!,$extra:String!,$n:Int!){
  main: search(query:$main,type:ISSUE,first:$n){ issueCount nodes{...Row} }
  extra: search(query:$extra,type:ISSUE,first:$n){ nodes{...Row} }
}${ROW}`;

function cleanLogin(input) {
  const s = (input ?? "").trim().replace(/^@/, "");
  const ok =
    s.length > 0 && s.length <= 39 && !s.startsWith("-") && !s.endsWith("-") && /^[A-Za-z0-9-]+$/.test(s);
  return ok ? s : null;
}

// the same chips the terminal draws, see crates/core/src/model.rs
function chipsFor(n) {
  const out = [];
  const merged = n.merged === true;
  const closed = n.state === "CLOSED";
  const open = !merged && !closed;

  if (merged) out.push({ text: "✔ merged", tone: "info" });
  else if (closed)
    out.push({ text: n.stateReason === "NOT_PLANNED" ? "✖ not planned" : "✖ closed", tone: "bad" });
  else if (n.isDraft === true) out.push({ text: "◐ draft", tone: "muted" });
  else out.push({ text: "● open", tone: "good" });

  if (!open) {
    if (n.reviewDecision === "APPROVED") out.push({ text: "✔ approved", tone: "good" });
    return out;
  }

  if (n.reviewDecision === "APPROVED") out.push({ text: "✔ approved", tone: "good" });
  else if (n.reviewDecision === "CHANGES_REQUESTED") out.push({ text: "✖ changes requested", tone: "bad" });
  else if (n.reviewDecision === "REVIEW_REQUIRED") out.push({ text: "○ needs review", tone: "warn" });

  if (n.mergeable === "CONFLICTING") out.push({ text: "! conflict", tone: "bad" });

  const ci = n.commits?.nodes?.[0]?.commit?.statusCheckRollup?.state;
  if (ci === "SUCCESS") out.push({ text: "✔ ci", tone: "good" });
  else if (ci === "FAILURE") out.push({ text: "✖ ci", tone: "bad" });
  else if (ci === "ERROR") out.push({ text: "! ci", tone: "bad" });
  else if (ci === "PENDING") out.push({ text: "◐ ci", tone: "warn" });

  if ((n.reviews?.totalCount ?? 0) > 0 && !n.reviewDecision) out.push({ text: "● reviewed", tone: "info" });

  for (const r of n.reviewRequests?.nodes ?? []) {
    const who = r?.requestedReviewer?.login ?? r?.requestedReviewer?.name;
    if (who) out.push({ text: `@${who}`, tone: "warn" });
  }
  return out;
}

function lastActivity(n) {
  let best = null;
  const consider = (at, who, what) => {
    if (!at || !who) return;
    const t = Date.parse(at);
    if (!Number.isNaN(t) && (!best || t > best.at)) best = { at: t, text: `${who} ${what}` };
  };
  for (const r of n.reviews?.nodes ?? []) {
    const what =
      r?.state === "APPROVED"
        ? "approved"
        : r?.state === "CHANGES_REQUESTED"
          ? "requested changes"
          : r?.state === "DISMISSED"
            ? "review dismissed"
            : "reviewed";
    consider(r?.submittedAt, r?.author?.login, what);
  }
  for (const c of n.comments?.nodes ?? []) consider(c?.createdAt, c?.author?.login, "commented");
  return best ? best.text : null;
}

function toRow(n, mentioned) {
  const kind = n?.__typename === "PullRequest" ? "pr" : n?.__typename === "Issue" ? "issue" : null;
  if (!kind || !n.repository?.nameWithOwner || typeof n.number !== "number") return null;
  return {
    kind,
    repo: n.repository.nameWithOwner,
    number: n.number,
    title: String(n.title ?? "").trim(),
    url: n.url,
    updatedAt: n.updatedAt,
    author: n.author?.login ?? "ghost",
    mentioned,
    open: !(n.merged === true || n.state === "CLOSED"),
    comments: n.comments?.totalCount ?? 0,
    activity: lastActivity(n),
    chips: chipsFor(n),
  };
}

function collect(nodes, mentioned, into) {
  for (const n of nodes ?? []) {
    const row = toRow(n, mentioned);
    if (!row) continue;
    const seen = into.get(row.url);
    if (seen) {
      if (mentioned) seen.mentioned = true;
    } else {
      into.set(row.url, row);
    }
  }
}

function rest(token, path) {
  return fetch(`${API}${path}`, {
    headers: {
      Authorization: `Bearer ${token}`,
      "User-Agent": "ghdeck-site",
      Accept: "application/vnd.github+json",
    },
  });
}

async function graphql(token, query, variables) {
  const res = await fetch(`${API}/graphql`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "User-Agent": "ghdeck-site",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query, variables }),
  });
  const body = await res.json().catch(() => null);
  if (!res.ok || !body?.data) {
    throw new Error(body?.errors?.[0]?.message ?? `github answered ${res.status}`);
  }
  return body.data;
}

// github hides some accounts from search, their public activity is the only way in
async function fromActivity(token, login) {
  const res = await rest(token, `/users/${login}/events/public?per_page=100`);
  if (!res.ok) return null;
  const events = await res.json();
  const kinds = {
    PullRequestEvent: "pull_request",
    PullRequestReviewEvent: "pull_request",
    PullRequestReviewCommentEvent: "pull_request",
    IssuesEvent: "issue",
    IssueCommentEvent: "issue",
  };
  const refs = [];
  const seen = new Set();
  let since = null;
  for (const e of events) {
    if (e?.created_at && (!since || e.created_at < since)) since = e.created_at;
    const key = kinds[e?.type];
    const number = key ? e?.payload?.[key]?.number : null;
    const repo = e?.repo?.name;
    if (!key || typeof number !== "number" || !repo) continue;
    const id = `${repo}#${number}`;
    if (!seen.has(id)) {
      seen.add(id);
      refs.push({ repo, number });
    }
  }

  const rows = new Map();
  for (let i = 0; i < Math.min(refs.length, FEED_ROWS); i += PER_PAGE) {
    const batch = refs.slice(i, i + PER_PAGE);
    const decl = [];
    const fields = [];
    const vars = {};
    batch.forEach((ref, k) => {
      const [owner, name] = ref.repo.split("/");
      if (!owner || !name) return;
      decl.push(`$o${k}:String!,$n${k}:String!,$k${k}:Int!`);
      fields.push(`r${k}: repository(owner:$o${k},name:$n${k}){issueOrPullRequest(number:$k${k}){...Row}}`);
      vars[`o${k}`] = owner;
      vars[`n${k}`] = name;
      vars[`k${k}`] = ref.number;
    });
    if (!decl.length) continue;
    const data = await graphql(token, `query(${decl.join(",")}){${fields.join("\n")}}${ROW}`, vars).catch(
      () => null,
    );
    if (!data) break;
    batch.forEach((_, k) => {
      const node = data[`r${k}`]?.issueOrPullRequest;
      if (node) collect([node], false, rows);
    });
  }
  return { rows: [...rows.values()], since };
}

const byNewest = (rows) => rows.sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));

function json(body, status, cacheable) {
  return new Response(JSON.stringify(body), {
    status: status ?? 200,
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": cacheable ? `public, max-age=${CACHE_SECONDS}` : "no-store",
    },
  });
}

// anything the cache hands back carries a browser ttl from the zone, so the
// copy the visitor gets is rebuilt and told to keep nothing
function forClient(res) {
  return new Response(res.body, {
    status: res.status,
    headers: { "Content-Type": "application/json", "Cache-Control": "no-store" },
  });
}

async function lookUp(url, env, ctx) {
  const login = cleanLogin(url.searchParams.get("name"));
  if (!login) return json({ error: "that is not a github username" }, 400);
  if (!env.GHDECK_TOKEN) return json({ error: "search is not set up on this site yet" }, 503);

  const cache = caches.default;
  const key = new Request(`${url.origin}/api/user?name=${login.toLowerCase()}`);
  const hit = await cache.match(key);
  if (hit) return forClient(hit);

  try {
    const data = await graphql(env.GHDECK_TOKEN, SEARCH, {
      main: `involves:${login} sort:updated-desc`,
      extra: `mentions:${login} sort:updated-desc`,
      n: PER_PAGE,
    });

    const rows = new Map();
    collect(data.main?.nodes, false, rows);
    collect(data.extra?.nodes, true, rows);

    let body = {
      login,
      rows: byNewest([...rows.values()]),
      total: data.main?.issueCount ?? 0,
      source: "search",
    };

    if (rows.size === 0) {
      const check = await rest(env.GHDECK_TOKEN, `/search/issues?q=author:${login}&per_page=1`);
      if (check.status === 422) {
        const who = await rest(env.GHDECK_TOKEN, `/users/${login}`);
        if (who.status === 404) return json({ error: `there is no github user called ${login}` }, 404);
        const feed = await fromActivity(env.GHDECK_TOKEN, login);
        if (feed) {
          body = { login, rows: byNewest(feed.rows), total: feed.rows.length, source: "activity", since: feed.since };
        }
      }
    }

    const res = json(body, 200, true);
    ctx.waitUntil(cache.put(key, res.clone()));
    return forClient(res);
  } catch (e) {
    const message = e instanceof Error ? e.message : "github did not answer";
    const busy = /rate limit|abuse|secondary/i.test(message);
    return json(
      { error: busy ? "github is rate limiting the site, try again in a minute" : message },
      busy ? 429 : 502,
    );
  }
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    if (url.pathname === "/api/user") {
      if (request.method !== "GET") return json({ error: "use GET" }, 405);
      return lookUp(url, env, ctx);
    }
    return env.ASSETS.fetch(request);
  },
};
