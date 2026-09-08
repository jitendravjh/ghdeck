---
layout: ../../layouts/DocsLayout.astro
title: Documentation
description: Install it, connect your account, and the keys you will actually use.
---

## Homebrew

macOS and Linux. Sorts out your PATH and upgrades for you.

```sh
brew install jitendravjh/tap/ghdeck
```

Upgrade with `brew upgrade ghdeck`, remove it with `brew uninstall ghdeck`.

## Download a binary

Nothing else needed. `uname -m` tells you which Mac you have, `arm64` is Apple Silicon and `x86_64` is Intel.

### macOS, Apple Silicon

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-aarch64-apple-darwin.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

### macOS, Intel

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-x86_64-apple-darwin.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

### Linux, x86_64

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

### Linux, arm64

```sh
curl -sSfL https://github.com/jitendravjh/ghdeck/releases/latest/download/ghdeck-aarch64-unknown-linux-musl.tar.gz | tar xz
sudo mv ghdeck /usr/local/bin/
```

Linux builds are static, so any distro works. Checksums are on the [release](https://github.com/jitendravjh/ghdeck/releases/latest). Downloading through a browser rather than curl leaves a macOS quarantine flag, cleared with `xattr -d com.apple.quarantine ghdeck`.

## From source

Only if you want to build it yourself. Needs Rust 1.88 and a C compiler.

```sh
cargo install --git https://github.com/jitendravjh/ghdeck ghdeck
```

## Connect your account

Pick one of these two, then run `ghdeck`.

### With gh

If you have [gh](https://cli.github.com), that is the whole thing. ghdeck borrows its token.

```sh
gh auth login
```

### With a token

Make one with the `repo` scope at [github.com/settings/tokens](https://github.com/settings/tokens), then put this in your `.zshrc` or `.bashrc` so it survives a new shell. gh is not needed this way.

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

## First run

```sh
ghdeck
```

The first run caches everything, a few seconds. After that it opens instantly and refreshes on its own. Windows is untested.

## Commands

```sh
ghdeck                                    # dashboard
ghdeck list                               # what needs attention
ghdeck list all                           # everything
ghdeck show JuliaGeometry/Meshes.jl#1428  # one conversation
ghdeck sync                               # refresh now
ghdeck --version
```

## Filters

`attention`, `open`, `yours`, `to-review` and `all`.

Attention means open items that are conflicting, have changes requested, have CI failing, or wait on your review.

## Keys

| key | does |
| --- | --- |
| up down | move |
| left right | switch filter |
| 1 to 5 | jump to a filter |
| enter | read the conversation |
| o | open in browser |
| y | copy url |
| r | sync |
| / | search |
| ? | help |
| q | quit |

In a conversation, up and down scroll, `b` expands bot messages, left or esc goes back.
