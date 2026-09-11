# ghdeck

### [ghdeck.jitendravjh.in](https://ghdeck.jitendravjh.in)

All your GitHub work in one list. PRs and issues together, newest first, with the status of each one sitting right there.

GitHub splits these across two pages and neither one tells you if a PR is conflicting or if CI has gone red. Other terminal dashboards keep PRs and issues in separate tabs. This one does not.

<img width="1074" height="448" alt="Screenshot 2026-09-11 at 10 38 52 AM" src="https://github.com/user-attachments/assets/bb758f14-479a-402b-9987-453b4f26d705" />


An issue closing and the PR that closed it, next to each other. Conflicts, failing CI and who you are waiting on all show up the same way.

## Install

Three ways. Pick one, then do [Setup](#setup).

### 1. Homebrew, on macOS or Linux

Easiest. Handles PATH and upgrades for you.

**You need Homebrew.** If `brew --version` gives nothing:

```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then:

```sh
brew install jitendravjh/tap/ghdeck
```

Upgrade later with `brew upgrade ghdeck`, uninstall with `brew uninstall ghdeck`.

### 2. Download a binary

**You need nothing else.**

Copy the block for your machine, there is nothing to fill in. Run `uname -m` if you are unsure which Mac you have: `arm64` is Apple Silicon, `x86_64` is Intel.

**macOS, Apple Silicon**

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-aarch64-apple-darwin.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

**macOS, Intel**

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-x86_64-apple-darwin.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

**Linux, x86_64**

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

**Linux, arm64**

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-aarch64-unknown-linux-musl.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

Linux builds are static, so any distro works. Checksums are in `SHA256SUMS` on the [release](https://github.com/jitendravjh/ghdeck/releases/latest).

To upgrade, run the same command again. To uninstall, `sudo rm /usr/local/bin/ghdeck`.

If you download through a browser rather than curl, macOS quarantines the file because the binary is not signed. Clear it with `xattr -d com.apple.quarantine ghdeck`.

### 3. From source

Only if you want to build it yourself. Needs Rust 1.88 or newer and a C compiler, since SQLite is built from source.

```sh
cargo install --git https://github.com/jitendravjh/ghdeck ghdeck
```

The binary lands in `~/.cargo/bin`, so keep that on your PATH.

## Setup

ghdeck needs GitHub credentials. Either way works, pick one.

**If you have [gh](https://cli.github.com):**

```sh
gh auth login
```

Nothing else to do, ghdeck borrows its token.

**Otherwise, a token.** Make one at [github.com/settings/tokens](https://github.com/settings/tokens) with the `repo` scope, then:

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

Put that line in your `.zshrc` or `.bashrc` so it survives a new shell. gh is not needed if you go this route.

Then run it:

```sh
ghdeck
```

The first run fetches everything and caches it, a few seconds. After that it opens instantly and refreshes in the background.

Windows is untested.

## Use

```sh
ghdeck                             # dashboard
ghdeck list                        # print what needs attention
ghdeck list all                    # print everything
ghdeck show JuliaGeometry/Meshes.jl#1428   # print one conversation
ghdeck sync                        # refresh now
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

Watching costs nothing. It polls notifications with `If-Modified-Since` and GitHub does not count 304s, so a quiet minute is free. `GHDECK_POLL_SECS` changes the interval if you want.

Everything sits in SQLite, so the dashboard opens on cached data and syncs behind you.

Bots get collapsed to one line in a conversation, otherwise codecov and CI comments bury the actual discussion.

## Not there yet

- GitHub search caps at 1000 results, so older history is out of reach
- No diffs
- Read only, no approve or merge

## Releasing

Bump `version` in `Cargo.toml`, commit, push. That is the whole thing.

```sh
git commit -am "Version 0.2.1" && git push
```

CI reads the version, sees the tag does not exist yet, and then builds all four
binaries, creates the tag and release, and pushes the formula to
[jitendravjh/homebrew-tap](https://github.com/jitendravjh/homebrew-tap). Push
without touching the version and nothing ships, so ordinary commits are safe.

The site is separate. It rebuilds on every push to `main` regardless of version.

MIT
