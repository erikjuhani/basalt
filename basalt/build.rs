use std::{env, path::Path, process::Command};

fn main() {
    let Some(commit_info) = commit_info() else {
        return;
    };

    let version = env::var("CARGO_PKG_VERSION").unwrap();

    println!(
        "cargo:rustc-env=BASALT_COMMIT_SHORT_HASH={}",
        commit_info.short_hash
    );
    println!("cargo:rustc-env=BASALT_COMMIT_HASH={}", commit_info.hash);
    println!("cargo:rustc-env=BASALT_COMMIT_DATE={}", commit_info.date);
    println!("cargo:rustc-env=BASALT_VERSION={version}");
}

struct CommitInfo {
    hash: String,
    short_hash: String,
    date: String,
}

fn commit_info() -> Option<CommitInfo> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent()?;

    // Not a git directory
    if !workspace_root.join(".git").exists() {
        return None;
    }

    // Reference for commit info output: https://github.com/rust-lang/cargo/blob/a967402/build.rs#L60
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .output()
    {
        Ok(output) if output.status.success() => Some(output),
        _ => None,
    }?;

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout = stdout.trim();
    let mut parts = stdout.split_whitespace().map(|s| s.to_string());

    let hash = parts.next()?;
    let short_hash = parts.next()?;
    let date = parts.next()?;

    Some(CommitInfo {
        hash,
        short_hash,
        date,
    })
}
