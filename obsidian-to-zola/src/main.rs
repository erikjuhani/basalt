//! Transforms the Obsidian vault under `docs/` into Zola-ready content
//! under `site/content/`, rewriting wiki-links and embeds along the way.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use regex::{Captures, Regex};
use serde::Deserialize;
use walkdir::WalkDir;

const EXCLUDED_DIRS: &[&str] = &[".obsidian"];
const LANDING_SOURCE: &str = "Home.md";
const CHANGELOG_SOURCE: &str = "basalt/CHANGELOG.md";
const CARGO_SOURCE: &str = "basalt/Cargo.toml";

#[derive(Debug, Default, Deserialize)]
struct Order {
    #[serde(default)]
    sections: HashMap<String, i64>,
    #[serde(default)]
    pages: HashMap<String, i64>,
}

struct Page {
    source_path: PathBuf,
    title: String,
    slug: String,
    /// Key used to look up weight in `_order.toml`'s `[pages]`, e.g.
    /// `User interface/Explorer` (no extension).
    order_key: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let strict = args.iter().any(|a| a == "--strict");

    let root = match repo_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("obsidian-to-zola: {e}");
            return ExitCode::from(2);
        }
    };

    match run(&root, strict) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("obsidian-to-zola: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(root: &Path, strict: bool) -> Result<(), String> {
    let docs = root.join("docs");
    let content = root.join("site").join("content");
    let assets_src = root.join("assets");
    let assets_dst = root.join("site").join("static").join("assets");

    reset_dir(&content)?;
    reset_dir(&assets_dst)?;
    copy_assets(&assets_src, &assets_dst)?;

    let order = read_order(&docs)?;
    let pages = collect_pages(&docs)?;
    let link_map = build_link_map(&pages);

    let re_wiki = Regex::new(r"(!?)\[\[([^\]\|]+?)(?:\|([^\]]+))?\]\]").unwrap();

    let mut warnings = 0usize;

    for page in &pages {
        let source_abs = docs.join(&page.source_path);
        let raw = fs::read_to_string(&source_abs)
            .map_err(|e| format!("read {}: {e}", source_abs.display()))?;

        let raw = strip_yaml_front_matter(&raw);
        let body = rewrite_body(&raw, &re_wiki, &link_map, &page.order_key, &mut warnings);
        let weight = order.pages.get(&page.order_key).copied().unwrap_or(10);
        let date = last_git_date(root, &source_abs).unwrap_or_else(today);

        let mut out = String::new();
        out.push_str("+++\n");
        out.push_str(&format!("title = {}\n", toml_str(&page.title)));
        out.push_str(&format!("date = \"{date}\"\n"));
        out.push_str(&format!("weight = {weight}\n"));
        if page.slug == "basalt" {
            out.push_str("path = \"/docs\"\n");
        }
        out.push_str("+++\n\n");
        out.push_str(&body);

        let out_path = content.join(format!("{}.md", page.slug));
        fs::create_dir_all(out_path.parent().unwrap())
            .map_err(|e| format!("mkdir for {}: {e}", out_path.display()))?;
        fs::write(&out_path, out).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    }

    // Emit _index.md for every section directory we saw.
    let mut sections: HashMap<String, String> = HashMap::new();
    for page in &pages {
        if let Some(parent) = page.source_path.parent() {
            if parent.as_os_str().is_empty() {
                continue;
            }
            let title = parent.to_string_lossy().into_owned();
            let slug = slugify_path(parent);
            sections.insert(slug, title);
        }
    }
    for (slug, title) in sections {
        let weight = order.sections.get(&title).copied().unwrap_or(10);
        let path = content.join(&slug).join("_index.md");
        fs::create_dir_all(path.parent().unwrap())
            .map_err(|e| format!("mkdir for {}: {e}", path.display()))?;
        let mut out = String::new();
        out.push_str("+++\n");
        out.push_str(&format!("title = {}\n", toml_str(&title)));
        out.push_str(&format!("weight = {weight}\n"));
        out.push_str("sort_by = \"weight\"\n");
        out.push_str("+++\n");
        fs::write(&path, out).map_err(|e| format!("write {}: {e}", path.display()))?;
    }

    let version = read_version(root)?;
    write_landing(&content, root, &version)?;
    write_changelog(&content, root, &version)?;
    copy_devlog(root, &content)?;
    patch_config_version(root, &version)?;

    println!(
        "obsidian-to-zola: wrote {} pages ({} warnings)",
        pages.len() + 1,
        warnings
    );
    if strict && warnings > 0 {
        return Err(format!("{} warnings (--strict)", warnings));
    }
    Ok(())
}

/// Detect repo root. Prefer `CARGO_MANIFEST_DIR` (crate sits at
/// `<root>/obsidian-to-zola`); fall back to the current directory.
fn repo_root() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let crate_dir = PathBuf::from(dir);
        if let Some(root) = crate_dir.ancestors().nth(1) {
            if root.join("docs").is_dir() {
                return Ok(root.to_path_buf());
            }
        }
    }
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    for ancestor in cwd.ancestors() {
        if ancestor.join("docs").is_dir() && ancestor.join("Cargo.toml").is_file() {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err("could not locate repo root (no docs/ in ancestors)".into())
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| format!("rm -rf {}: {e}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|e| format!("mkdir {}: {e}", path.display()))
}

fn copy_assets(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("{} is not a directory", src.display()));
    }
    for entry in WalkDir::new(src) {
        let entry = entry.map_err(|e| format!("walk {}: {e}", src.display()))?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| format!("mkdir {}: {e}", target.display()))?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), &target).map_err(|e| {
                format!(
                    "copy {} -> {}: {e}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn copy_devlog(root: &Path, content: &Path) -> Result<(), String> {
    let src = root.join("devlog");
    if !src.is_dir() {
        return Ok(());
    }
    let dst = content.join("devlog");
    fs::create_dir_all(&dst).map_err(|e| format!("mkdir {}: {e}", dst.display()))?;
    for entry in fs::read_dir(&src).map_err(|e| format!("read {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("read entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = path.file_name().unwrap();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let target = dst.join(name);
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let rewritten = if stem == "_index" {
            raw
        } else {
            inject_devlog_slug(&raw, stem)
        };
        fs::write(&target, rewritten).map_err(|e| format!("write {}: {e}", target.display()))?;
    }
    Ok(())
}

/// Preserves the `YYYY-MM-DD-` prefix in devlog URLs by forcing the slug
/// to match the filename stem. Zola would otherwise strip the date part.
fn inject_devlog_slug(src: &str, stem: &str) -> String {
    if !src.starts_with("+++") {
        return src.to_string();
    }
    let rest = &src[3..];
    let Some(end) = rest.find("+++") else {
        return src.to_string();
    };
    let front = &rest[..end];
    let tail = &rest[end..];
    if front.lines().any(|l| l.trim_start().starts_with("slug")) {
        return src.to_string();
    }
    let trimmed = front.trim_end_matches('\n');
    format!("+++{trimmed}\nslug = \"{stem}\"\n{tail}")
}

fn read_order(docs: &Path) -> Result<Order, String> {
    let path = docs.join("_order.toml");
    if !path.exists() {
        return Ok(Order::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn collect_pages(docs: &Path) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    let walker = WalkDir::new(docs).follow_links(true);
    for entry in walker.into_iter().filter_entry(keep_entry) {
        let entry = entry.map_err(|e| format!("walk {}: {e}", docs.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") || name.starts_with('_') || name == LANDING_SOURCE {
            continue;
        }
        let rel = path.strip_prefix(docs).unwrap().to_path_buf();
        let title = name.trim_end_matches(".md").to_string();
        let slug = slugify_path(&strip_ext(&rel));
        let order_key = rel
            .with_extension("")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        pages.push(Page {
            source_path: rel,
            title,
            slug,
            order_key,
        });
    }
    Ok(pages)
}

fn keep_entry(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if entry.depth() == 0 {
        return true;
    }
    if entry.file_type().is_dir() {
        !EXCLUDED_DIRS.iter().any(|d| *d == name.as_ref())
    } else {
        true
    }
}

fn strip_ext(path: &Path) -> PathBuf {
    if let Some(stem) = path.file_stem() {
        path.with_file_name(stem)
    } else {
        path.to_path_buf()
    }
}

fn slugify_path(path: &Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(slugify)
        .collect::<Vec<_>>()
        .join("/")
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for ch in s.chars() {
        match ch.to_ascii_lowercase() {
            ch if ch.is_ascii_alphanumeric() => {
                out.push(ch);
                prev_dash = false;
            }
            ' ' | '_' | '-' if !prev_dash && !out.is_empty() => {
                out.push('-');
                prev_dash = true;
            }
            // Strip everything else (parens, punctuation).
            _ => {}
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn build_link_map(pages: &[Page]) -> HashMap<String, String> {
    // Use Zola's internal link syntax (`@/section/page.md`) so links stay
    // correct under any `base_url`, including project-page subpaths.
    let mut map = HashMap::new();
    for page in pages {
        let target = format!("@/{}.md", page.slug);
        map.insert(page.title.clone(), target.clone());
        map.insert(page.order_key.clone(), target);
    }
    map
}

fn rewrite_body(
    raw: &str,
    re: &Regex,
    link_map: &HashMap<String, String>,
    ctx: &str,
    warnings: &mut usize,
) -> String {
    // Split the document on fenced code blocks so we don't rewrite inside them.
    let mut out = String::with_capacity(raw.len());
    let mut in_fence = false;
    for line in raw.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        let rewritten = re.replace_all(line, |caps: &Captures<'_>| {
            rewrite_wiki(caps, link_map, ctx, warnings)
        });
        out.push_str(&rewritten);
    }
    out
}

fn rewrite_wiki(
    caps: &Captures<'_>,
    link_map: &HashMap<String, String>,
    ctx: &str,
    warnings: &mut usize,
) -> String {
    let bang = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let target = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
    let alt = caps.get(3).map(|m| m.as_str().trim());

    if bang == "!" {
        // Embed: ![[file.ext]]
        let ext = Path::new(target)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "gif" => {
                let name = Path::new(target)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(target);
                format!("{{{{ gif(name=\"{}\") }}}}", name)
            }
            "png" | "jpg" | "jpeg" | "webp" | "svg" => {
                format!("![]({})", asset_url(target))
            }
            _ => {
                eprintln!("warn [{ctx}]: unsupported embed target `{target}`");
                *warnings += 1;
                caps.get(0).unwrap().as_str().to_string()
            }
        }
    } else {
        // Link: [[Target]] or [[Target|Alt]]
        let url = link_map.get(target);
        let display = alt.unwrap_or(target);
        match url {
            Some(u) => format!("[{display}]({u})"),
            None => {
                eprintln!("warn [{ctx}]: unresolved wiki-link `{target}`");
                *warnings += 1;
                display.to_string()
            }
        }
    }
}

fn asset_url(target: &str) -> String {
    // `![[explorer.png]]` → /assets/explorer.png. Images don't live in
    // dark/light pairs, so they land at /assets/ directly.
    format!("/assets/{target}")
}

fn strip_yaml_front_matter(raw: &str) -> String {
    if !raw.starts_with("---\n") && !raw.starts_with("---\r\n") {
        return raw.to_string();
    }
    let rest = raw
        .trim_start_matches("---\n")
        .trim_start_matches("---\r\n");
    if let Some(idx) = rest.find("\n---\n").or_else(|| rest.find("\n---\r\n")) {
        let after = &rest[idx..];
        let after = after
            .trim_start_matches("\n---\n")
            .trim_start_matches("\n---\r\n");
        return after.trim_start_matches('\n').to_string();
    }
    raw.to_string()
}

fn last_git_date(root: &Path, file: &Path) -> Option<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["log", "-1", "--format=%aI", "--"])
        .arg(file)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn today() -> String {
    // Zola accepts YYYY-MM-DD; we can't pull chrono just for this, so emit
    // a fixed but valid date when git has nothing to say.
    "1970-01-01".to_string()
}

fn toml_str(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn write_landing(content: &Path, root: &Path, _version: &str) -> Result<(), String> {
    // Landing front matter lives in a hand-maintained file so we don't
    // have to bake hero content into the transform.
    let landing_src = root.join("site").join("_landing.md");
    if !landing_src.exists() {
        return Ok(());
    }
    let body = fs::read_to_string(&landing_src)
        .map_err(|e| format!("read {}: {e}", landing_src.display()))?;
    let dest = content.join("_index.md");
    fs::write(&dest, body).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(())
}

fn write_changelog(content: &Path, root: &Path, version: &str) -> Result<(), String> {
    let src = root.join(CHANGELOG_SOURCE);
    if !src.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&src).map_err(|e| format!("read {}: {e}", src.display()))?;
    // Strip the leading `# Changelog` so we don't render two H1s (title + body).
    let body = raw
        .lines()
        .skip_while(|l| l.trim().is_empty() || l.trim().eq_ignore_ascii_case("# changelog"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = String::new();
    out.push_str("+++\n");
    out.push_str("title = \"Changelog\"\n");
    out.push_str(&format!("date = \"{}\"\n", today()));
    out.push_str("weight = 90\n");
    out.push_str("+++\n\n");
    out.push_str(&body);
    let dest = content.join("changelog.md");
    fs::write(&dest, out).map_err(|e| format!("write {}: {e}", dest.display()))?;
    let _ = version;
    Ok(())
}

fn read_version(root: &Path) -> Result<String, String> {
    let path = root.join(CARGO_SOURCE);
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("version") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let v = rest.trim().trim_matches('"').to_string();
                if !v.is_empty() {
                    return Ok(v);
                }
            }
        }
    }
    Err(format!("no version field found in {}", path.display()))
}

fn patch_config_version(root: &Path, version: &str) -> Result<(), String> {
    let path = root.join("site").join("config.toml");
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let marker = "basalt_version";
    let mut patched = String::with_capacity(raw.len());
    let mut replaced = false;
    for line in raw.split_inclusive('\n') {
        if !replaced && line.trim_start().starts_with(marker) {
            patched.push_str(&format!("{marker} = \"{version}\"\n"));
            replaced = true;
        } else {
            patched.push_str(line);
        }
    }
    if !replaced {
        // Append under [extra] if not present already.
        if !patched.ends_with('\n') {
            patched.push('\n');
        }
        if !raw.contains("[extra]") {
            patched.push_str("\n[extra]\n");
        }
        patched.push_str(&format!("{marker} = \"{version}\"\n"));
    }
    fs::write(&path, patched).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("User interface"), "user-interface");
        assert_eq!(slugify("Editor (experimental)"), "editor-experimental");
        assert_eq!(slugify("Known Limitations"), "known-limitations");
        assert_eq!(slugify("Files and Folders"), "files-and-folders");
    }

    #[test]
    fn rewrite_wiki_link() {
        let re = Regex::new(r"(!?)\[\[([^\]\|]+?)(?:\|([^\]]+))?\]\]").unwrap();
        let mut link_map = HashMap::new();
        link_map.insert("Explorer".into(), "@/user-interface/explorer.md".into());
        let mut warnings = 0;
        let body = "See [[Explorer]] and [[Explorer|file list]].";
        let out = rewrite_body(body, &re, &link_map, "test", &mut warnings);
        assert_eq!(
            out,
            "See [Explorer](@/user-interface/explorer.md) and [file list](@/user-interface/explorer.md)."
        );
        assert_eq!(warnings, 0);
    }

    #[test]
    fn rewrite_gif_embed() {
        let re = Regex::new(r"(!?)\[\[([^\]\|]+?)(?:\|([^\]]+))?\]\]").unwrap();
        let link_map = HashMap::new();
        let mut warnings = 0;
        let out = rewrite_body("![[explorer.gif]]", &re, &link_map, "test", &mut warnings);
        assert!(out.contains("{{ gif(name=\"explorer\") }}"));
        assert_eq!(warnings, 0);
    }

    #[test]
    fn skip_code_fences() {
        let re = Regex::new(r"(!?)\[\[([^\]\|]+?)(?:\|([^\]]+))?\]\]").unwrap();
        let link_map = HashMap::new();
        let mut warnings = 0;
        let body = "Before [[X]]\n```\n[[X]] stays\n```\nAfter [[X]]";
        let out = rewrite_body(body, &re, &link_map, "test", &mut warnings);
        assert!(out.contains("[[X]] stays"));
        // Two outside-fence lookups fail and become plain text:
        assert_eq!(warnings, 2);
    }
}
