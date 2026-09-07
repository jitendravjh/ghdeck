# ghwork

All your GitHub work in one list. PRs and issues together, newest first, with the status of each one sitting right there.

GitHub splits these across two pages and neither one tells you if a PR is conflicting or if CI has gone red. Other terminal dashboards keep PRs and issues in separate tabs. This one does not.

```
  2h  PR  kubernetes-sigs/headlamp#7128  i18n: Complete Hindi translations
      open · changes requested · ci running · waiting on vyncent-t   4 comments  kubernetes-prow commented

 3mo  PR  SciML/NeuralOperators.jl#134  feat: implement ConvolutionalNeuralOperator
      open · conflict   2 comments  jitendravjh commented
```

## Install

Needs `gh` logged in, or a `GITHUB_TOKEN` in the environment.

```sh
cargo install --path crates/cli
```

## Use

```sh
ghwork                             # dashboard
ghwork list                        # print what needs you
ghwork list all                    # print everything
ghwork show calcom/cal.diy#29686   # print one conversation
ghwork sync                        # refresh now
```

Filters are `needs-you`, `open`, `mine`, `to-review` and `all`.

`needs-you` means open items that are conflicting, have changes requested, have CI failing, or are waiting on your review.

## Keys

```
j k         move
enter       read the conversation
tab, 1 to 5 switch filter
o           open in browser
y           copy url
r           sync
/           search
?           help
q           quit
```

Inside a conversation `j k` scrolls, `b` expands bot messages and `esc` takes you back.

## How it stays cheap

One GraphQL query gets PRs and issues interleaved and already sorted. A full sync of 130 items costs about 18 points out of 5000 an hour.

Watching costs nothing. It polls notifications with `If-Modified-Since` and GitHub does not count 304s, so a quiet minute is free. `GHWORK_POLL_SECS` changes the interval if you want.

Everything sits in SQLite, so the dashboard opens on cached data and syncs behind you.

Bots get collapsed to one line in a conversation, otherwise codecov and CI comments bury the actual discussion.

## Not there yet

- GitHub search caps at 1000 results, so older history is out of reach
- No diffs
- Read only, no approve or merge

MIT
