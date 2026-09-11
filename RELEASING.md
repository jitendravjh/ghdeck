# Releasing

Bump `version` in `Cargo.toml`, commit, push. That is the whole thing.

```sh
git commit -am "Version 0.2.3" && git push
```

CI reads the version, sees the tag does not exist yet, then builds the macOS and Linux binaries, creates the tag and release, and pushes the formula to [jitendravjh/homebrew-tap](https://github.com/jitendravjh/homebrew-tap). The Windows build runs last in its own job, so if it ever breaks, everything else still ships. Push without touching the version and nothing ships, so ordinary commits are safe.

To check the Windows installer after a release, run the windows workflow by hand from the Actions tab. It installs the new release on a Windows machine and runs it.

The site is separate. It rebuilds on every push to `main` regardless of version.
