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

## Install script

One line for macOS and Linux, nothing else needed. It picks the right build for your machine, checks it against the release checksums, and puts it in `/usr/local/bin`. It asks for your password only if that folder needs it.

```sh
curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | sh
```

Linux builds are static, so any distro works. To install somewhere else, put the folder before `sh`:

```sh
curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | GHDECK_BIN_DIR="$HOME/.local/bin" sh
```

Or pin a version the same way:

```sh
curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | GHDECK_VERSION=v0.2.0 sh
```

## Windows

Run this in PowerShell. It puts ghdeck in `%LOCALAPPDATA%\Programs\ghdeck` and adds that folder to your PATH, no admin needed.

```powershell
irm https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.ps1 | iex
```

Windows is new and untested. If you are on Windows, give it a go and [open an issue](https://github.com/jitendravjh/ghdeck/issues) saying what works and what doesn't.

## From source

Only if you want to build it yourself. Needs Rust 1.88 and a C compiler.

```sh
cargo install --git https://github.com/jitendravjh/ghdeck ghdeck
```

## Update

With Homebrew:

```sh
brew upgrade ghdeck
```

With the install script, run it again. It replaces the old binary with the newest release.

```sh
curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | sh
```

On Windows, run the installer again:

```powershell
irm https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.ps1 | iex
```

From source:

```sh
cargo install --git https://github.com/jitendravjh/ghdeck ghdeck --force
```

## Uninstall

With Homebrew:

```sh
brew uninstall ghdeck
```

With the install script:

```sh
sudo rm /usr/local/bin/ghdeck
```

On Windows, then take the folder off your PATH:

```powershell
Remove-Item "$env:LOCALAPPDATA\Programs\ghdeck" -Recurse
```

From source:

```sh
cargo uninstall ghdeck
```

The cache stays behind in `~/Library/Application Support/ghdeck` on macOS, `~/.local/share/ghdeck` on Linux and `%APPDATA%\ghdeck` on Windows, delete that folder too for a clean removal. If you put `GITHUB_TOKEN` in your `.zshrc` or `.bashrc`, take that line out as well.

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

On Windows, run this once and open a new terminal:

```powershell
setx GITHUB_TOKEN ghp_your_token_here
```

## First run

```sh
ghdeck
```

The first run caches everything, a few seconds. Everything sits in SQLite. Cached data shows straight away while a sync runs behind you.

## Commands

```sh
ghdeck                                    # dashboard
ghdeck list                               # what needs attention
ghdeck list all                           # everything
ghdeck show JuliaGeometry/Meshes.jl#1428  # one conversation
ghdeck user [username]                    # anyone's prs and issues
ghdeck sync                               # refresh now
ghdeck --version
```

## Filters

`attention`, `open`, `yours`, `to-review` and `all`.

Attention means open items that are conflicting, have changes requested, have CI failing, or wait on your review.

## Other people

Press `u` in the dashboard and type a username, or run `ghdeck user [username]`. You get their PRs and issues newest first, with the same status on every row. Esc takes you back to your own list.

Filters there are `all`, `authored`, `mentioned` and `open`. You only see what your own token can see, so their work in private repos you have no access to won't show up.

GitHub keeps some accounts out of search. For those, ghdeck shows their public activity instead. That feed only holds their last 300 events, so for a busy person it covers a couple of weeks, and mentions are not in it.

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
| u | someone else's prs and issues |
| ? | help |
| q | quit |

In a conversation, up and down scroll, `b` expands bot messages, left or esc goes back.
