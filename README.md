# ghwork

All your GitHub work in one list. PRs and issues together, newest first, with the status of each one sitting right there.

GitHub splits these across two pages and neither one tells you if a PR is conflicting or if CI has gone red. Other terminal dashboards keep PRs and issues in separate tabs. This one does not.

```
ATTENTION NEEDED [7]   OPEN [13]   YOURS [111]   TO REVIEW [0]   ALL [130]

 54m  [ISSUE] SciML/SciMLBenchmarks.jl#126  Fix AdaptiveSDE benchmarks
      closed   5 comments  jitendravjh commented
 54m  [PR]    SciML/SciMLBenchmarks.jl#1605  Fix and update AdaptiveSDE benchmarks (#126)
      merged   37 comments  ChrisRackauckas commented
  1h  [PR]    JuliaGeometry/Meshes.jl#1429  Add Geodesic primitive
      draft · ci fail · reviewed   3 comments  jitendravjh reviewed
  5h  [PR]    kubernetes-sigs/headlamp#7128  i18n: Complete Hindi translations for app and glossary
      open · changes requested · ci running · waiting on vyncent-t   4 comments  kubernetes-prow commented
```

An issue closing and the PR that closed it, next to each other. Conflicts, failing CI and who you are waiting on all show up the same way.

## Install

What you need first:

- **Rust 1.88 or newer** for `cargo`, its build tool. `brew install rust`, or [rustup](https://rustup.rs).
- **A C compiler**, since SQLite is compiled from source. `xcode-select --install` on macOS, `gcc` or `clang` on Linux.
- **[gh](https://cli.github.com) logged in**, or a `GITHUB_TOKEN` in the environment with the `repo` scope.

Then:

```sh
cargo install --git https://github.com/jitendravjh/ghwork ghwork
```

The binary lands in `~/.cargo/bin`, so keep that on your PATH.

Built and used on macOS. Linux should be fine. Windows is untested, and the clipboard key will not work there.

## Use

```sh
ghwork                             # dashboard
ghwork list                        # print what needs attention
ghwork list all                    # print everything
ghwork show calcom/cal.diy#29686   # print one conversation
ghwork sync                        # refresh now
```

Filters are `attention`, `open`, `yours`, `to-review` and `all`.

Attention needed means open items that are conflicting, have changes requested, have CI failing, or are waiting on your review.

## Keys

```
up down       move
left right    switch filter
1 to 5        jump straight to a filter
enter         read the conversation
o             open in browser
y             copy url
r             sync
/             search
?             help
q             quit
```

Inside a conversation, up and down scroll, `b` expands bot messages, and left or esc takes you back.

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
