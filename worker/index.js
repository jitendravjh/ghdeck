// Serves the built site, and answers /api/user with someone's PRs and issues.
// The token lives in the GHDECK_TOKEN secret, browsers never see it.

import lookupPage from "../site/dist/u/index.html";
import labPage from "../site/dist/experimental/index.html";

const API = "https://api.github.com";
const PER_PAGE = 20;
const FEED_ROWS = 40;
const CACHE_SECONDS = 300;

// the body is only asked for when a page wants to preview it, it is the one heavy field
const row = (rich) => `
fragment Row on SearchResultItem{
  ... on Issue{ __typename number title url state stateReason createdAt updatedAt
    repository{nameWithOwner} author{login} assignees(first:10){nodes{login}}
    labels(first:8){nodes{name color}}${rich ? " bodyText" : ""}
    comments(last:1){totalCount nodes{author{login} createdAt}} }
  ... on PullRequest{ __typename number title url state isDraft createdAt updatedAt mergedAt
    repository{nameWithOwner} author{login} assignees(first:10){nodes{login}}
    labels(first:8){nodes{name color}}${rich ? " bodyText" : ""}
    merged mergeable reviewDecision
    comments(last:1){totalCount nodes{author{login} createdAt}}
    reviews(last:10){totalCount nodes{state author{login} submittedAt}}
    reviewRequests(first:8){nodes{requestedReviewer{... on User{login}}}}
    commits(last:1){nodes{commit{statusCheckRollup{state}}}} }
}`;

const search = (rich) => `
query($main:String!,$extra:String!,$n:Int!,$am:String,$ae:String){
  main: search(query:$main,type:ISSUE,first:$n,after:$am){
    issueCount pageInfo{endCursor hasNextPage} nodes{...Row}
  }
  extra: search(query:$extra,type:ISSUE,first:$n,after:$ae){
    pageInfo{endCursor hasNextPage} nodes{...Row}
  }
}${row(rich)}`;

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

function toRow(n, mentioned, login) {
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
    assigned: (n.assignees?.nodes ?? []).some((a) => a?.login?.toLowerCase() === login.toLowerCase()),
    open: !(n.merged === true || n.state === "CLOSED"),
    merged: n.merged === true,
    draft: n.isDraft === true,
    createdAt: n.createdAt ?? null,
    mergedAt: n.mergedAt ?? null,
    labels: (n.labels?.nodes ?? []).map((l) => ({ name: l.name, color: l.color })),
    body: n.bodyText ? n.bodyText.trim().slice(0, 400) : null,
    comments: n.comments?.totalCount ?? 0,
    activity: lastActivity(n),
    chips: chipsFor(n),
  };
}

function collect(nodes, mentioned, into, login) {
  for (const n of nodes ?? []) {
    const row = toRow(n, mentioned, login);
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

const CALENDAR = `
query($login:String!,$from:DateTime!,$to:DateTime!){
  user(login:$login){
    contributionsCollection(from:$from,to:$to){
      contributionCalendar{
        totalContributions
        weeks{contributionDays{date contributionCount}}
      }
    }
  }
}`;

// the real contribution graph, so the calendar matches their github profile
// instead of only the pull requests and issues this search can see
async function calendarOf(token, login, year) {
  const now = new Date();
  const asked = year ? new Date(Date.UTC(year, 11, 31, 23, 59, 59)) : now;
  const to = asked > now ? now : asked;
  const from = year
    ? new Date(Date.UTC(year, 0, 1))
    : new Date(Date.UTC(now.getUTCFullYear() - 1, now.getUTCMonth(), now.getUTCDate() + 1));
  const data = await graphql(token, CALENDAR, {
    login,
    from: from.toISOString(),
    to: to.toISOString(),
  }).catch(() => null);
  const box = data?.user?.contributionsCollection?.contributionCalendar;
  if (!box) return null;
  const days = {};
  for (const week of box.weeks ?? []) {
    for (const day of week.contributionDays ?? []) {
      if (day.contributionCount > 0) days[day.date] = day.contributionCount;
    }
  }
  return {
    total: box.totalContributions ?? 0,
    from: from.toISOString().slice(0, 10),
    to: to.toISOString().slice(0, 10),
    days,
  };
}

async function profileOf(token, login) {
  const res = await rest(token, `/users/${login}`);
  if (!res.ok) return null;
  const who = await res.json();
  return {
    login: who.login,
    name: who.name || null,
    avatar: who.avatar_url || null,
    url: who.html_url || null,
    joined: who.created_at || null,
    repos: who.public_repos ?? null,
    location: who.location || null,
  };
}

// github hides some accounts from search, their public activity is the only way in
async function fromActivity(token, login, skip = 0, rich = false) {
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
  const wanted = refs.slice(skip, skip + FEED_ROWS);
  for (let i = 0; i < wanted.length; i += PER_PAGE) {
    const batch = wanted.slice(i, i + PER_PAGE);
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
    const data = await graphql(token, `query(${decl.join(",")}){${fields.join("\n")}}${row(rich)}`, vars).catch(
      () => null,
    );
    if (!data) break;
    batch.forEach((_, k) => {
      const node = data[`r${k}`]?.issueOrPullRequest;
      if (node) collect([node], false, rows, login);
    });
  }
  return { rows: [...rows.values()], since, more: refs.length > skip + FEED_ROWS };
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

// one year of the calendar on its own, for the picker on the analytics panel
async function oneYear(url, env, ctx) {
  const login = cleanLogin(url.searchParams.get("name"));
  if (!login) return json({ error: "that is not a github username" }, 400);
  if (!env.GHDECK_TOKEN) return json({ error: "search is not set up on this site yet" }, 503);
  const asked = Number(url.searchParams.get("year"));
  const year = Number.isInteger(asked) && asked >= 2008 && asked <= 2100 ? asked : null;

  const cache = caches.default;
  const key = new Request(`${url.origin}/api/calendar?name=${login.toLowerCase()}&year=${year ?? ""}`);
  const hit = await cache.match(key);
  if (hit) return forClient(hit);

  const calendar = await calendarOf(env.GHDECK_TOKEN, login, year);
  if (!calendar) return json({ error: "github did not answer" }, 502);
  const res = json({ login, calendar }, 200, true);
  ctx.waitUntil(cache.put(key, res.clone()));
  return forClient(res);
}

async function lookUp(url, env, ctx) {
  const login = cleanLogin(url.searchParams.get("name"));
  if (!login) return json({ error: "that is not a github username" }, 400);
  if (!env.GHDECK_TOKEN) return json({ error: "search is not set up on this site yet" }, 503);

  // a page beyond the first carries the cursors github gave us for each search
  const am = url.searchParams.get("am") || null;
  const ae = url.searchParams.get("ae") || null;
  const skip = Math.max(0, Math.min(300, Number(url.searchParams.get("skip")) || 0));
  const rich = url.searchParams.get("rich") === "1";
  const size = Math.max(1, Math.min(50, Number(url.searchParams.get("size")) || PER_PAGE));

  const cache = caches.default;
  const page = am || ae ? `&am=${am ?? ""}&ae=${ae ?? ""}` : skip ? `&skip=${skip}` : "";
  const key = new Request(
    `${url.origin}/api/user?name=${login.toLowerCase()}${page}&size=${size}${rich ? "&rich=1" : ""}`,
  );
  const hit = await cache.match(key);
  if (hit) return forClient(hit);

  try {
    // the profile and the calendar are their own calls, so they run beside the search
    const first = !(am || ae || skip);
    const profilePromise = first ? profileOf(env.GHDECK_TOKEN, login) : Promise.resolve(null);
    const calendarPromise = first ? calendarOf(env.GHDECK_TOKEN, login, null) : Promise.resolve(null);
    const data = await graphql(env.GHDECK_TOKEN, search(rich), {
      main: `involves:${login} sort:updated-desc`,
      extra: `mentions:${login} sort:updated-desc`,
      n: size,
      am,
      ae,
    });

    const rows = new Map();
    collect(data.main?.nodes, false, rows, login);
    collect(data.extra?.nodes, true, rows, login);

    let body = {
      login,
      rows: byNewest([...rows.values()]),
      total: data.main?.issueCount ?? 0,
      source: "search",
      profile: await profilePromise,
      calendar: await calendarPromise,
      next: {
        am: data.main?.pageInfo?.endCursor ?? null,
        ae: data.extra?.pageInfo?.endCursor ?? null,
        more: Boolean(data.main?.pageInfo?.hasNextPage || data.extra?.pageInfo?.hasNextPage),
      },
    };

    // only the search path uses cursors, so a cursor means this is not a hidden account
    if (!am && !ae && (rows.size === 0 || skip > 0)) {
      const check = await rest(env.GHDECK_TOKEN, `/search/issues?q=author:${login}&per_page=1`);
      if (check.status === 422) {
        const who = await rest(env.GHDECK_TOKEN, `/users/${login}`);
        if (who.status === 404) return json({ error: `there is no github user called ${login}` }, 404);
        const feed = await fromActivity(env.GHDECK_TOKEN, login, skip, rich);
        if (feed) {
          body = {
            login,
            rows: byNewest(feed.rows),
            total: feed.rows.length,
            source: "activity",
            since: feed.since,
            profile: await profilePromise,
            calendar: await calendarPromise,
            next: { skip: skip + FEED_ROWS, more: feed.more },
          };
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

// /u/<username> and /experimental/<username> are shareable links, each page
// reads the name off its own path. /user/ was the old spelling and still serves.
const USER_PATH = /^\/(?:u|user)(?:\/([A-Za-z0-9-]{1,39}))?\/?$/;
const LAB_PATH = /^\/experimental(?:\/([A-Za-z0-9-]{1,39}))?\/?$/;
const CARD_PATH = /^\/og\/u\/([A-Za-z0-9-]{1,39})\.png$/;
// bump when the card changes, so the edge stops handing out the old drawing
const CARD_DRAW = 2;

const tagEscape = (s) =>
  String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

// every username is served the same bundled page, so the link preview has to be
// written in per request. only the name is needed, the picture does the rest.
function personalise(html, login, origin) {
  const url = `${origin}/u/${login}`;
  const image = `${origin}/og/u/${login}.png`;
  const title = tagEscape(`@${login} on ghdeck`);
  const about = tagEscape(
    `Pull requests and issues ${login} is involved in, newest first, the way ghdeck shows them.`,
  );
  return html
    .replace(/<title>[^<]*<\/title>/, `<title>${title}</title>`)
    .replace(/(<link rel="canonical" href=")[^"]*/, `$1${url}`)
    .replace(/(<meta name="description" content=")[^"]*/, `$1${about}`)
    .replace(/(<meta property="og:url" content=")[^"]*/, `$1${url}`)
    .replace(/(<meta property="og:title" content=")[^"]*/, `$1${title}`)
    .replace(/(<meta property="og:description" content=")[^"]*/, `$1${about}`)
    .replace(/(<meta property="og:image" content=")[^"]*/, `$1${image}`)
    .replace(/(<meta property="og:image:alt" content=")[^"]*/, `$1${tagEscape(`${login} on ghdeck`)}`)
    .replace(/(<meta name="twitter:title" content=")[^"]*/, `$1${title}`)
    .replace(/(<meta name="twitter:description" content=")[^"]*/, `$1${about}`)
    .replace(/(<meta name="twitter:image" content=")[^"]*/, `$1${image}`);
}

async function cardFor(url, env, ctx, login) {
  if (!env.GHDECK_TOKEN) return env.ASSETS.fetch(new Request(`${url.origin}/og.png`));

  const cache = caches.default;
  const key = new Request(`${url.origin}/og/u/${login.toLowerCase()}.png?draw=${CARD_DRAW}`);
  const hit = await cache.match(key);
  if (hit) return hit;

  let made = null;
  try {
    const { userCard } = await import("./card.js");
    made = await userCard(login, { profileOf, graphql, token: env.GHDECK_TOKEN });
  } catch {
    made = null;
  }
  // a preview is never worth a broken link, the site card stands in
  if (!made) return env.ASSETS.fetch(new Request(`${url.origin}/og.png`));

  const png = new Response(made.body, {
    headers: { "Content-Type": "image/png", "Cache-Control": "public, max-age=86400" },
  });
  ctx.waitUntil(cache.put(key, png.clone()));
  return png;
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    if (url.pathname === "/api/user") {
      if (request.method !== "GET") return json({ error: "use GET" }, 405);
      return lookUp(url, env, ctx);
    }
    if (url.pathname === "/api/calendar") {
      if (request.method !== "GET") return json({ error: "use GET" }, 405);
      return oneYear(url, env, ctx);
    }
    const wantsCard = CARD_PATH.exec(url.pathname);
    if (wantsCard) {
      const login = cleanLogin(wantsCard[1]);
      if (!login) return env.ASSETS.fetch(new Request(`${url.origin}/og.png`));
      return cardFor(url, env, ctx, login);
    }
    // the assets binding will not serve a rewritten path to a browser navigation,
    // so the page is bundled in and returned directly
    const asUser = USER_PATH.exec(url.pathname);
    const asLab = LAB_PATH.exec(url.pathname);
    if (asUser || asLab) {
      const login = cleanLogin((asLab ?? asUser)[1] ?? "");
      let page = asLab ? labPage : lookupPage;
      if (login) page = personalise(page, login, url.origin);
      return new Response(page, {
        headers: { "Content-Type": "text/html;charset=utf-8", "Cache-Control": "public, max-age=0, must-revalidate" },
      });
    }
    return env.ASSETS.fetch(request);
  },
};
