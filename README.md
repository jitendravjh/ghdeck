<h1 align="left">
  <a href="https://ghdeck.jitendravjh.in">
    <img
      src="https://readme-typing-svg.demolab.com/?font=Fira+Code&weight=600&size=40&duration=3000&pause=1000&color=08C225&vCenter=true&width=600&height=70&lines=ghdeck"
      alt="ghdeck"
    />
  </a>
</h1>

### [ghdeck.jitendravjh.in](https://ghdeck.jitendravjh.in)

All your GitHub work in one list. PRs and issues together, newest first, with the status of each one sitting right there.

GitHub splits these across two pages and neither one tells you if a PR is conflicting or if CI has gone red. Other terminal dashboards keep PRs and issues in separate tabs. This one does not.

<img width="1074" height="448" alt="Screenshot 2026-09-11 at 10 38 52 AM" src="https://github.com/user-attachments/assets/bb758f14-479a-402b-9987-453b4f26d705" />

An issue closing and the PR that closed it, next to each other. Conflicts, failing CI and who you are waiting on all show up the same way. Press `u` and you can look at anyone else's work like this too.

## Install

Pick one, then do [Setup](#setup).

### On macOS or Linux

**Homebrew:** (If you don't have Homebrew, install it first, from [brew.sh](https://brew.sh/).

```sh
brew install jitendravjh/tap/ghdeck
```

**Install script:** It picks the right build for your machine, checks it against the release checksums and puts it in `/usr/local/bin`, asking for your password only if that folder needs it.

```sh
curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | sh
```
Linux builds are static, so any distro works.

### Windows

Run this in PowerShell. It puts ghdeck in `%LOCALAPPDATA%\Programs\ghdeck` and adds that folder to your PATH, no admin needed.

```powershell
irm https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.ps1 | iex
```

Windows is new and untested. I don't have a Windows machine, so if you do, give it a go and [open an issue](https://github.com/jitendravjh/ghdeck/issues) saying what works and what doesn't. A screenshot helps a lot.

### From source

Needs Rust 1.88 or newer and a C compiler, since SQLite is built from source.

```sh
cargo install --git https://github.com/jitendravjh/ghdeck ghdeck
```

The binary lands in `~/.cargo/bin`, so keep that on your PATH.

### Update and uninstall

| Installed with | Update | Uninstall |
| --- | --- | --- |
| Homebrew | `brew upgrade ghdeck` | `brew uninstall ghdeck` |
| Install script | run the same command again | `sudo rm /usr/local/bin/ghdeck` |
| Windows | run the same command again | `Remove-Item "$env:LOCALAPPDATA\Programs\ghdeck" -Recurse` |
| Source | `cargo install --git https://github.com/jitendravjh/ghdeck ghdeck --force` | `cargo uninstall ghdeck` |

The cache stays behind in `~/Library/Application Support/ghdeck` on macOS, `~/.local/share/ghdeck` on Linux and `%APPDATA%\ghdeck` on Windows, so delete that too for a clean removal. On Windows, also take the folder off your PATH.

## Setup

ghdeck needs GitHub credentials. Pick one.

**If you have [gh](https://cli.github.com):**

```sh
gh auth login
```

Nothing else to do, ghdeck borrows its token.

**Otherwise, a token.** Make one at [github.com/settings/tokens](https://github.com/settings/tokens) with the `repo` scope. On macOS or Linux, put this in your `.zshrc` or `.bashrc` so it survives a new shell:

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

On Windows, run this once and open a new terminal:

```powershell
setx GITHUB_TOKEN ghp_your_token_here
```

Then run it:

```sh
ghdeck
```

The first run caches everything, a few seconds. After that it opens straight away from the cache while a sync runs behind you.

## Use

```sh
ghdeck                                    # dashboard
ghdeck list                               # print what needs attention
ghdeck list all                           # print everything
ghdeck show JuliaGeometry/Meshes.jl#1428  # print one conversation
ghdeck user [username]                    # print anyone's prs and issues
ghdeck sync                               # refresh now
```

Filters are `attention`, `open`, `yours`, `to-review` and `all`. Attention needed means open items that are conflicting, have changes requested, have CI failing, or are waiting on your review.

### Someone else's work

Press `u` in the dashboard and type a username, or run `ghdeck user [username]`. You get their PRs and issues newest first with the same status on every row, and esc takes you back to yours. Their filters are `all`, `authored`, `mentioned` and `open`, and you only see what your own token can see.

GitHub keeps some accounts out of search. For those, ghdeck shows their public activity instead, which covers their last 300 events.

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
u             someone else's prs and issues, esc to come back
?             help
q             quit
```

Inside a conversation, up and down scroll, `b` expands bot messages, and left or esc takes you back.

## Watching costs nothing

One GraphQL query gets PRs and issues interleaved and already sorted. A full sync of 130 items costs about 18 points out of 5000 an hour.

It polls notifications with `If-Modified-Since` and GitHub does not count 304s, so a quiet minute is free. `GHDECK_POLL_SECS` changes the interval if you want.

Bots get collapsed to one line in a conversation, otherwise codecov and CI comments bury the actual discussion.

## MIT LICENSED

MIT, see [LICENSE](LICENSE).
