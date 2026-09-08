#!/usr/bin/env bash
# Prints the Homebrew formula for a given release tag, eg: script/formula.sh v0.1.0
set -euo pipefail

tag="${1:?usage: formula.sh <tag>}"
repo="${GHWORK_REPO:-jitendravjh/ghwork}"
base="https://github.com/$repo/releases/download/$tag"

sums="$(curl -sSfL "$base/SHA256SUMS")"
sum_for() {
  echo "$sums" | awk -v f="ghwork-$1.tar.gz" '$2 == f { print $1 }'
}

mac_arm="$(sum_for aarch64-apple-darwin)"
mac_x86="$(sum_for x86_64-apple-darwin)"
linux_arm="$(sum_for aarch64-unknown-linux-musl)"
linux_x86="$(sum_for x86_64-unknown-linux-musl)"

for name in mac_arm mac_x86 linux_arm linux_x86; do
  [ -n "${!name}" ] || { echo "no checksum for $name in $tag" >&2; exit 1; }
done

cat <<RB
class Ghwork < Formula
  desc "All your GitHub work in one list, PRs and issues together sorted by last update"
  homepage "https://github.com/$repo"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/ghwork-aarch64-apple-darwin.tar.gz"
      sha256 "$mac_arm"
    end
    on_intel do
      url "$base/ghwork-x86_64-apple-darwin.tar.gz"
      sha256 "$mac_x86"
    end
  end

  on_linux do
    on_arm do
      url "$base/ghwork-aarch64-unknown-linux-musl.tar.gz"
      sha256 "$linux_arm"
    end
    on_intel do
      url "$base/ghwork-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$linux_x86"
    end
  end

  def install
    bin.install "ghwork"
  end

  test do
    assert_match "all your github work", shell_output("#{bin}/ghwork --help")
  end
end
RB
