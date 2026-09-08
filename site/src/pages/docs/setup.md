---
layout: ../../layouts/DocsLayout.astro
title: Setup
description: Sign in with gh, or set a token.
---

Pick one, then run `ghdeck`.

## With gh

If you have [gh](https://cli.github.com), that is the whole thing. ghdeck borrows its token.

```sh
gh auth login
```

## With a token

Make one with the `repo` scope at [github.com/settings/tokens](https://github.com/settings/tokens), then put this in your `.zshrc` or `.bashrc` so it survives a new shell. gh is not needed this way.

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

## First run

```sh
ghdeck
```

The first run caches everything, a few seconds. After that it opens instantly and refreshes on its own. Windows is untested.

Next, [the commands and keys](/docs/use/).
