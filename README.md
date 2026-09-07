# ghwork

All your GitHub work in one list. PRs and issues together, newest first, with the status of each one sitting right there.

GitHub splits these across two pages and neither one tells you if a PR is conflicting or if CI has gone red. Other terminal dashboards keep PRs and issues in separate tabs. This one does not.

```
 23m  IS  SciML/SciMLBenchmarks.jl#126  Fix AdaptiveSDE benchmarks
      closed   5 comments  jitendravjh commented
 23m  PR  SciML/SciMLBenchmarks.jl#1605  Fix and update AdaptiveSDE benchmarks (#126)
      merged   37 comments  ChrisRackauckas commented
 38m  PR  JuliaGeometry/Meshes.jl#1429  Add Geodesic primitive
      draft · ci fail · reviewed   3 comments  jitendravjh reviewed
  4h  PR  kubernetes-sigs/headlamp#7128  i18n: Complete Hindi translations for app and glossary
      open · changes requested · ci running · waiting on vyncent-t   4 comments  kubernetes-prow commented
```

An issue closing and the PR that closed it, next to each other. Conflicts, failing CI and who you are waiting on all show up the same way.

## Install

`cargo` is Rust's build tool. `brew install rust` if you do not have it.

```sh
cargo install --git https://github.com/jitendravjh/ghwork ghwork
```

That drops the binary in `~/.cargo/bin`, so put it on your PATH.

You also need `gh` logged in, or a `GITHUB_TOKEN` in the environment.

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
