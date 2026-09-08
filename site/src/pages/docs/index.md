---
layout: ../../layouts/DocsLayout.astro
title: Install
description: Homebrew, a prebuilt binary, or from source.
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

Next, [connect your account](/docs/setup/).
