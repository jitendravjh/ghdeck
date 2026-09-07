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

### Download a binary

No Rust needed. macOS on Apple Silicon:

```sh
curl -sSfL https://github.com/jitendravjh/ghwork/releases/latest/download/ghwork-aarch64-apple-darwin.tar.gz | tar xz
sudo mv ghwork /usr/local/bin/
```

Same thing for the rest, just swap the file name:

| platform | file |
| --- | --- |
| macOS, Apple Silicon | `ghwork-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `ghwork-x86_64-apple-darwin.tar.gz` |
| Linux, x86_64 | `ghwork-x86_64-unknown-linux-musl.tar.gz` |
| Linux, arm64 | `ghwork-aarch64-unknown-linux-musl.tar.gz` |

Linux builds are static, so any distro works. Checksums are in `SHA256SUMS` on the [release](https://github.com/jitendravjh/ghwork/releases/latest).

If you grab it through a browser rather than curl, macOS will quarantine it since the binary is not signed. Clear that with `xattr -d com.apple.quarantine ghwork`.

### Build from source

Needs Rust 1.88 or newer and a C compiler, because SQLite is built from source.

```sh
# macOS
brew install rust && xcode-select --install
# Debian or Ubuntu
sudo apt install cargo build-essential
```

Then:

```sh
cargo install --git https://github.com/jitendravjh/ghwork ghwork
```

The binary lands in `~/.cargo/bin`, so keep that on your PATH.

## Setup

ghwork needs GitHub credentials. Either of these works.

If you have [gh](https://cli.github.com):

```sh
gh auth login
```

Otherwise a token, with the `repo` scope, from [github.com/settings/tokens](https://github.com/settings/tokens):

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

Put that in your `.zshrc` or `.bashrc` so it survives a new shell. gh is not required if you go this route.

Then just run it:

```sh
ghwork
```

The first run fetches everything and caches it, a few seconds. After that it opens instantly and refreshes in the background.

Windows is untested.

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
