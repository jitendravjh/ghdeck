use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn token() -> Result<String> {
    for key in ["GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return Ok(v.trim().to_string());
            }
        }
    }
    let out = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("gh not found, install it or set GITHUB_TOKEN")?;
    if !out.status.success() {
        bail!("gh auth token failed, run `gh auth login`");
    }
    let t = String::from_utf8(out.stdout)?.trim().to_string();
    if t.is_empty() {
        bail!("no token from gh, run `gh auth login`");
    }
    Ok(t)
}
