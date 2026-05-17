use std::{env, path::Path, process::Command};

fn main() {
    let Some(commit) = commit_info_from_git().or_else(commit_info_from_source_tarball) else {
        return;
    };

    println!("cargo:rustc-env=BASALT_COMMIT_HASH={}", commit.hash);
    println!(
        "cargo:rustc-env=BASALT_COMMIT_SHORT_HASH={}",
        commit.short_hash
    );
    println!("cargo:rustc-env=BASALT_COMMIT_DATE={}", commit.date);
}

struct CommitInfo {
    hash: String,
    short_hash: String,
    date: String,
}

// Reference: https://github.com/rust-lang/cargo/blob/91fbe9/build.rs#L60-L116
fn commit_info_from_git() -> Option<CommitInfo> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let workspace_root = Path::new(&manifest_dir).parent()?;

    if !workspace_root.join(".git").exists() {
        return None;
    }

    let output = match Command::new("git")
        .current_dir(workspace_root)
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return None,
    };

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut parts = stdout.split_whitespace().map(|s| s.to_string());

    Some(CommitInfo {
        hash: parts.next()?,
        short_hash: parts.next()?,
        date: parts.next()?,
    })
}

// Source tarballs published to crates.io don't include a `.git` directory, so `git log` can't
// populate the commit hash and date at build time.
//
// To work around this, the release workflow writes a `git-commit-info` file before `cargo publish`
// packages the crate; cargo bundles the file into the tarball, and this function reads it on
// consumer builds.
//
// The file is a newline-separated list of full commit hash, short commit hash, and commit date.
// Reference: https://github.com/rust-lang/cargo/blob/91fbe9/build.rs#L60-L116
fn commit_info_from_source_tarball() -> Option<CommitInfo> {
    let path = Path::new("git-commit-info");
    if !path.exists() {
        return None;
    }

    // Dependency tracking is a nice to have for this (git doesn't do it), so if the path is not
    // valid UTF-8 just avoid doing it rather than erroring out.
    if let Some(utf8) = path.to_str() {
        println!("cargo:rerun-if-changed={utf8}");
    }

    let content = std::fs::read_to_string(path).ok()?;
    let mut parts = content.split('\n').map(|s| s.to_string());

    Some(CommitInfo {
        hash: parts.next()?,
        short_hash: parts.next()?,
        date: parts.next()?,
    })
}
