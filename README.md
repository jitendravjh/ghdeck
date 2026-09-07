# ghwork

All your GitHub work in one list. Pull requests and issues together, newest first, with the status of each one visible without opening anything.

GitHub splits this across two pages and neither shows you whether a PR is conflicting or its CI has gone red. Existing terminal dashboards keep PRs and issues in separate tabs. This keeps them in one stream.

## What a row looks like

```
  2h  PR  kubernetes-sigs/headlamp#7128  i18n: Complete Hindi translations
      open · changes requested · ci running · waiting on vyncent-t   4 comments  kubernetes-prow commented

 3mo  PR  SciML/NeuralOperators.jl#134  feat: implement ConvolutionalNeuralOperator
      open · conflict   2 comments  jitendravjh commented
```

## Install

Needs the `gh` CLI logged in, or a `GITHUB_TOKEN` in the environment.

```sh
cargo install --path crates/cli
```

## Use

```sh
ghwork              # dashboard
ghwork list         # print the needs-you list
ghwork list all     # print everything
ghwork sync         # refresh the cache
ghwork poll         # refresh only if something actually changed
ghwork where        # path to the cache
```

```sh
ghwork show calcom/cal.diy#29686   # print the conversation
```

Filters are `needs-you`, `open`, `mine`, `to-review`, `all`.

`needs-you` means open items that are conflicting, have changes requested, have failing CI, or are waiting on your review.

## Keys

```
j k, arrows      move
g G              top, bottom
tab, shift-tab   cycle filter
1 to 5           jump to filter
enter, l         read the conversation
o                open in browser
y                copy url
r                sync
R                deep sync, more pages
/                search title, repo, label
?                help
q                quit
```

## How it stays cheap

One GraphQL query gets both PRs and issues interleaved and already sorted, using `involves:@me`. A second query covers review requests, since `involves` does not include those. A full sync of 130 items costs about 18 points out of 5000 per hour.

Change detection is free. `GET /notifications` with `If-Modified-Since` returns 304 when nothing has moved, and GitHub does not count 304s against the rate limit. So `ghwork poll` costs nothing on a quiet repo and only spends points when there is actually something new.

The dashboard watches for changes on its own using that poll, on whatever interval GitHub asks for in `X-Poll-Interval`, usually 60 seconds. Four auto-polls over 40 seconds measured zero points spent on either the GraphQL or the REST bucket. Set `GHWORK_POLL_SECS` to override the interval.

Everything is cached in SQLite, so the dashboard opens on cached data straight away and syncs in the background. One worker thread owns the connection and the token, so nothing re-authenticates per refresh.

## Reading a thread

Press enter on a row to read it. The PR body, comments and reviews come in on one query, sorted oldest first, with line comments attached under the review that made them.

Bots are collapsed to a single line each, because codecov and CI bots otherwise bury the actual conversation. Press `b` to expand them. Markdown links are reduced to their text so a collapsed line stays readable.

## Layout

```
crates/core   auth, graphql, cache, sync
crates/cli    ratatui dashboard and the plain list
```

The core has no UI dependency, so a GUI can sit on the same data later.

## Known gaps

- GitHub search caps at 1000 results, so very old history is not reachable
- `mergeable` comes back unknown while GitHub computes it, those rows need a re-sync to settle
- No write actions yet, it is read only
- Diffs are not shown, only the conversation

## Licence

MIT
