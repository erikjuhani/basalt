//! This module provides functionality operating with Obsidian workspace state.
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::hash_map::RandomState,
    collections::HashSet,
    fs,
    hash::BuildHasher,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::obsidian::{Error, Result};

/// Represents the Obsidian workspace state, typically loaded from a `workspace.json` file
/// inside a vault's `.obsidian` directory.
///
/// More info: [https://help.obsidian.md/data-storage#Vault+settings]
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workspace {
    active: Option<String>,
    last_open_files: Vec<PathBuf>,
    main: Option<Node>,
}

impl Workspace {
    /// Returns the identifier of the active workspace leaf, if any.
    ///
    /// The identifier refers to a leaf in the workspace layout, not a file path.
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Returns the vault-relative paths of the most recently open files, most recent first.
    pub fn last_open_files(&self) -> &[PathBuf] {
        &self.last_open_files
    }

    /// Vault-relative paths of the notes open as tabs in the main editor area, in tab order.
    pub fn open_files(&self) -> Vec<&Path> {
        self.main.iter().flat_map(Node::markdown_files).collect()
    }

    /// Vault-relative path of the note open in the active tab, if the active leaf is a note.
    pub fn active_file(&self) -> Option<&Path> {
        let active = self.active.as_deref()?;
        self.main.as_ref()?.find_leaf(active)?.markdown_file()
    }
}

/// Obsidian's split/tabs/leaf layout tree. A leaf holds any view Obsidian supports.
/// Only [`ViewState::Markdown`] holds a file path.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Node {
    Split {
        children: Vec<Node>,
    },
    Tabs {
        children: Vec<Node>,
    },
    Leaf {
        id: String,
        state: ViewState,
    },
    #[serde(other)]
    Unknown,
}

impl Node {
    fn markdown_files(&self) -> Vec<&Path> {
        match self {
            Node::Split { children } | Node::Tabs { children } => {
                children.iter().flat_map(Node::markdown_files).collect()
            }
            _ => self.markdown_file().into_iter().collect(),
        }
    }

    fn find_leaf(&self, id: &str) -> Option<&Node> {
        match self {
            Node::Split { children } | Node::Tabs { children } => {
                children.iter().find_map(|child| child.find_leaf(id))
            }
            Node::Leaf { id: leaf_id, .. } if leaf_id == id => Some(self),
            _ => None,
        }
    }

    fn markdown_file(&self) -> Option<&Path> {
        match self {
            Node::Leaf { state, .. } => state.markdown_file(),
            _ => None,
        }
    }
}

/// The view a leaf displays. Basalt only distinguishes notes from everything else.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ViewState {
    Markdown {
        state: MarkdownState,
    },
    #[serde(other)]
    Other,
}

impl ViewState {
    fn markdown_file(&self) -> Option<&Path> {
        match self {
            ViewState::Markdown { state } => Some(&state.file),
            ViewState::Other => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct MarkdownState {
    file: PathBuf,
}

/// Attempts to load `workspace.json` from the given `.obsidian` directory path.
///
/// Returns an [`Error`] if the file doesn't exist or JSON parsing failed.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian;
///
/// _ = obsidian::workspace::load_from(Path::new("./dir-with-workspace-file"));
/// ```
pub fn load_from(config_dir: &Path) -> Result<Workspace> {
    let workspace_json_path = config_dir.join("workspace.json");
    if workspace_json_path.try_exists()? {
        let contents = fs::read_to_string(workspace_json_path)?;
        serde_json::from_str(&contents).map_err(Error::Json)
    } else {
        Err(Error::PathNotFound(
            workspace_json_path.to_string_lossy().to_string(),
        ))
    }
}

/// Writes `open_files`/`active_file` (vault-relative paths, in tab order) into `workspace.json`,
/// preserving every field Basalt doesn't model, such as sidebars or a leaf's scroll position.
/// Walks the raw JSON tree instead of round-tripping through [`Node`], since that typed model
/// would drop those fields.
///
/// Does nothing if `config_dir` doesn't exist. Creates `workspace.json` if it's missing.
pub fn save_to(config_dir: &Path, open_files: &[&Path], active_file: Option<&Path>) -> Result<()> {
    if !config_dir.try_exists()? {
        return Ok(());
    }

    let workspace_json_path = config_dir.join("workspace.json");
    let mut root = if workspace_json_path.try_exists()? {
        let contents = fs::read_to_string(&workspace_json_path)?;
        serde_json::from_str(&contents).map_err(Error::Json)?
    } else {
        json!({})
    };
    if !root.is_object() {
        root = json!({});
    }
    let original = root.clone();

    {
        let main = root
            .as_object_mut()
            .expect("root was just made an object")
            .entry("main")
            .or_insert_with(default_main);
        reconcile_tabs(main, open_files);
    }

    if let Some(active_file) = active_file {
        if let Some(id) = root
            .get("main")
            .and_then(|main| find_leaf_id(main, active_file))
        {
            root["active"] = json!(id);
        }
        front_recent_file(&mut root, active_file);
    }

    if root == original {
        return Ok(());
    }

    let contents = serde_json::to_string_pretty(&root).map_err(Error::Json)?;
    let tmp_path = config_dir.join("workspace.json.basalt-tmp");
    fs::write(&tmp_path, contents)?;
    fs::rename(tmp_path, workspace_json_path)?;
    Ok(())
}

fn default_main() -> Value {
    json!({
        "id": new_leaf_id(),
        "type": "split",
        "children": [{ "id": new_leaf_id(), "type": "tabs", "children": [] }],
        "direction": "vertical",
    })
}

fn reconcile_tabs(main: &mut Value, open_files: &[&Path]) {
    let keep: HashSet<&Path> = open_files.iter().copied().collect();
    prune_closed(main, &keep);

    let already_open = markdown_files(main);
    let new_files = open_files
        .iter()
        .copied()
        .filter(|path| !already_open.contains(*path));

    if let Some(children) = first_tabs_group(main)
        .and_then(|group| group.get_mut("children"))
        .and_then(Value::as_array_mut)
    {
        children.extend(new_files.map(new_markdown_leaf));
    }

    normalize_tabs_groups(main);
}

fn prune_closed(node: &mut Value, keep: &HashSet<&Path>) {
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children.iter_mut() {
        prune_closed(child, keep);
    }
    children.retain(|child| match json_markdown_file(child) {
        Some(file) => keep.contains(file.as_path()),
        None => true,
    });
}

/// Replaces an emptied tabs group with Obsidian's own placeholder leaf, since an empty
/// group isn't a shape Obsidian itself ever writes.
fn normalize_tabs_groups(node: &mut Value) {
    let is_tabs_group = node.get("type").and_then(Value::as_str) == Some("tabs");
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children.iter_mut() {
        normalize_tabs_groups(child);
    }
    if is_tabs_group {
        children.retain(|child| leaf_view_type(child) != Some("empty"));
        if children.is_empty() {
            children.push(empty_leaf());
        }
    }
}

fn first_tabs_group(node: &mut Value) -> Option<&mut Value> {
    if node.get("type").and_then(Value::as_str) == Some("tabs") {
        return Some(node);
    }
    node.get_mut("children")?
        .as_array_mut()?
        .iter_mut()
        .find_map(first_tabs_group)
}

fn markdown_files(node: &Value) -> HashSet<PathBuf> {
    let mut files = HashSet::new();
    collect_markdown_files(node, &mut files);
    files
}

fn collect_markdown_files(node: &Value, files: &mut HashSet<PathBuf>) {
    if let Some(file) = json_markdown_file(node) {
        files.insert(file);
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_markdown_files(child, files);
        }
    }
}

fn find_leaf_id(node: &Value, file: &Path) -> Option<String> {
    if json_markdown_file(node).as_deref() == Some(file) {
        return node.get("id")?.as_str().map(String::from);
    }
    node.get("children")?
        .as_array()?
        .iter()
        .find_map(|child| find_leaf_id(child, file))
}

fn leaf_view_type(node: &Value) -> Option<&str> {
    if node.get("type").and_then(Value::as_str) != Some("leaf") {
        return None;
    }
    node.get("state")?.get("type")?.as_str()
}

fn json_markdown_file(node: &Value) -> Option<PathBuf> {
    if leaf_view_type(node) != Some("markdown") {
        return None;
    }
    let file = node.get("state")?.get("state")?.get("file")?.as_str()?;
    Some(PathBuf::from(file))
}

fn new_markdown_leaf(path: &Path) -> Value {
    let title = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    json!({
        "id": new_leaf_id(),
        "type": "leaf",
        "state": {
            "type": "markdown",
            "state": { "file": path.to_string_lossy(), "mode": "source", "source": false },
            "icon": "lucide-file",
            "title": title,
        }
    })
}

fn empty_leaf() -> Value {
    json!({
        "id": new_leaf_id(),
        "type": "leaf",
        "state": { "type": "empty", "state": {}, "icon": "lucide-file" },
    })
}

fn front_recent_file(root: &mut Value, active_file: &Path) {
    let entry = root
        .as_object_mut()
        .expect("root is always an object here")
        .entry("lastOpenFiles")
        .or_insert_with(|| json!([]));
    let Some(files) = entry.as_array_mut() else {
        return;
    };
    let active = active_file.to_string_lossy().into_owned();
    files.retain(|file| file.as_str() != Some(active.as_str()));
    files.insert(0, json!(active));
}

/// A 16 hex character id in the same shape Obsidian generates for a new leaf.
fn new_leaf_id() -> String {
    format!("{:016x}", RandomState::new().hash_one(SystemTime::now()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_active_and_last_open_files() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            r#"{"active":"29458dac1350a42f","lastOpenFiles":["index.md"]}"#,
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), Some("29458dac1350a42f"));
        assert_eq!(workspace.last_open_files(), [PathBuf::from("index.md")]);
    }

    #[test]
    fn ignores_unknown_fields() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            r#"{"left":{},"active":"leaf-id","lastOpenFiles":[]}"#,
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), Some("leaf-id"));
        assert!(workspace.last_open_files().is_empty());
    }

    const MAIN_WITH_TABS: &str = r#"{
        "id": "split-1",
        "type": "split",
        "children": [
            {
                "id": "tabs-1",
                "type": "tabs",
                "children": [
                    {
                        "id": "leaf-a",
                        "type": "leaf",
                        "state": {
                            "type": "markdown",
                            "state": { "file": "notes/a.md", "mode": "source" },
                            "icon": "lucide-file",
                            "title": "a"
                        }
                    },
                    {
                        "id": "leaf-empty",
                        "type": "leaf",
                        "state": { "type": "empty", "state": {}, "icon": "lucide-file" }
                    },
                    {
                        "id": "leaf-b",
                        "type": "leaf",
                        "state": {
                            "type": "markdown",
                            "state": { "file": "b.md", "mode": "source" },
                            "icon": "lucide-file",
                            "title": "b"
                        }
                    }
                ]
            }
        ],
        "direction": "vertical"
    }"#;

    #[test]
    fn open_files_lists_markdown_tabs_in_order() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            format!(r#"{{"main":{MAIN_WITH_TABS}}}"#),
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(
            workspace.open_files(),
            [Path::new("notes/a.md"), Path::new("b.md")]
        );
    }

    #[test]
    fn active_file_resolves_active_leaf_id() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            format!(r#"{{"active":"leaf-b","main":{MAIN_WITH_TABS}}}"#),
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active_file(), Some(Path::new("b.md")));
    }

    #[test]
    fn active_file_is_none_for_non_markdown_leaf() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            format!(r#"{{"active":"leaf-empty","main":{MAIN_WITH_TABS}}}"#),
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active_file(), None);
    }

    #[test]
    fn open_files_is_empty_without_main() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), "{}").unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert!(workspace.open_files().is_empty());
    }

    #[test]
    fn unknown_node_type_does_not_fail_parsing() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            r#"{"main":{"id":"mosaic-1","type":"mosaic","children":[]}}"#,
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert!(workspace.open_files().is_empty());
    }

    #[test]
    fn defaults_missing_properties() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), "{}").unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), None);
        assert!(workspace.last_open_files().is_empty());
    }

    #[test]
    fn errors_when_file_is_missing() {
        let dir = tempdir().unwrap();

        assert!(matches!(load_from(dir.path()), Err(Error::PathNotFound(_))));
    }

    #[test]
    fn errors_when_file_is_malformed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), "not json").unwrap();

        assert!(matches!(load_from(dir.path()), Err(Error::Json(_))));
    }

    const SIMPLE_TWO_TAB_WORKSPACE: &str = r#"{
        "main": {
            "id": "split-1",
            "type": "split",
            "children": [
                {
                    "id": "tabs-1",
                    "type": "tabs",
                    "children": [
                        {
                            "id": "leaf-a",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "a.md", "mode": "source" },
                                "icon": "lucide-file",
                                "title": "a"
                            }
                        },
                        {
                            "id": "leaf-b",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "b.md", "mode": "source" },
                                "icon": "lucide-file",
                                "title": "b"
                            }
                        }
                    ]
                }
            ],
            "direction": "vertical"
        },
        "active": "leaf-a"
    }"#;

    const WORKSPACE_WITH_SIDEBARS: &str = r#"{
        "main": {
            "id": "split-1",
            "type": "split",
            "children": [
                {
                    "id": "tabs-1",
                    "type": "tabs",
                    "children": [
                        {
                            "id": "leaf-a",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "notes/a.md", "mode": "source", "scroll": 12 },
                                "icon": "lucide-file",
                                "title": "a"
                            }
                        },
                        {
                            "id": "leaf-b",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "b.md", "mode": "source" },
                                "icon": "lucide-file",
                                "title": "b"
                            }
                        },
                        {
                            "id": "leaf-canvas",
                            "type": "leaf",
                            "state": {
                                "type": "canvas",
                                "state": { "file": "board.canvas" },
                                "icon": "lucide-layout-dashboard"
                            }
                        }
                    ]
                }
            ],
            "direction": "vertical"
        },
        "left": {
            "id": "left-split",
            "type": "split",
            "children": [
                {
                    "id": "left-tabs",
                    "type": "tabs",
                    "children": [
                        {
                            "id": "explorer-leaf",
                            "type": "leaf",
                            "state": { "type": "file-explorer", "state": {}, "icon": "lucide-folder-closed" }
                        }
                    ]
                }
            ],
            "direction": "horizontal",
            "width": 300
        },
        "somePluginState": { "foo": "bar" },
        "active": "leaf-a",
        "lastOpenFiles": ["b.md", "notes/a.md", "image.png"]
    }"#;

    const TWO_GROUP_WORKSPACE: &str = r#"{
        "main": {
            "id": "outer-split",
            "type": "split",
            "children": [
                {
                    "id": "group-1",
                    "type": "tabs",
                    "children": [
                        {
                            "id": "leaf-a",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "a.md", "mode": "source" },
                                "icon": "lucide-file",
                                "title": "a"
                            }
                        }
                    ]
                },
                {
                    "id": "group-2",
                    "type": "tabs",
                    "children": [
                        {
                            "id": "leaf-b",
                            "type": "leaf",
                            "state": {
                                "type": "markdown",
                                "state": { "file": "b.md", "mode": "source" },
                                "icon": "lucide-file",
                                "title": "b"
                            }
                        }
                    ]
                }
            ],
            "direction": "horizontal"
        },
        "active": "leaf-a"
    }"#;

    #[test]
    fn save_to_prunes_and_adds_across_tab_groups() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), TWO_GROUP_WORKSPACE).unwrap();

        save_to(
            dir.path(),
            &[Path::new("a.md"), Path::new("c.md")],
            Some(Path::new("a.md")),
        )
        .unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let groups = saved["main"]["children"].as_array().unwrap();
        let group_1 = groups[0]["children"].as_array().unwrap();
        let group_2 = groups[1]["children"].as_array().unwrap();

        assert!(group_1
            .iter()
            .any(|leaf| leaf["state"]["state"]["file"] == "a.md"));
        assert!(group_1
            .iter()
            .any(|leaf| leaf["state"]["state"]["file"] == "c.md"));
        assert_eq!(group_2.len(), 1);
        assert_eq!(group_2[0]["state"]["type"], "empty");
    }

    #[test]
    fn save_to_preserves_fields_it_does_not_model() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), WORKSPACE_WITH_SIDEBARS).unwrap();

        save_to(
            dir.path(),
            &[Path::new("notes/a.md"), Path::new("b.md")],
            Some(Path::new("notes/a.md")),
        )
        .unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let original: Value = serde_json::from_str(WORKSPACE_WITH_SIDEBARS).unwrap();

        assert_eq!(saved["left"], original["left"]);
        assert_eq!(saved["somePluginState"], original["somePluginState"]);
    }

    #[test]
    fn save_to_keeps_an_unchanged_leaf_untouched() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), WORKSPACE_WITH_SIDEBARS).unwrap();

        save_to(
            dir.path(),
            &[Path::new("notes/a.md"), Path::new("b.md")],
            None,
        )
        .unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let original: Value = serde_json::from_str(WORKSPACE_WITH_SIDEBARS).unwrap();
        let leaves = saved["main"]["children"][0]["children"].as_array().unwrap();
        let leaf_a = leaves.iter().find(|leaf| leaf["id"] == "leaf-a").unwrap();

        assert_eq!(leaf_a, &original["main"]["children"][0]["children"][0]);
    }

    #[test]
    fn save_to_adds_new_tabs_and_removes_closed_ones() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), WORKSPACE_WITH_SIDEBARS).unwrap();

        save_to(
            dir.path(),
            &[Path::new("notes/a.md"), Path::new("c.md")],
            Some(Path::new("c.md")),
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();
        assert_eq!(
            workspace.open_files(),
            [Path::new("notes/a.md"), Path::new("c.md")]
        );
        assert_eq!(workspace.active_file(), Some(Path::new("c.md")));

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let leaves = saved["main"]["children"][0]["children"].as_array().unwrap();
        assert!(leaves.iter().any(|leaf| leaf["id"] == "leaf-canvas"));
    }

    #[test]
    fn save_to_generates_a_16_hex_char_id_for_a_new_leaf() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), SIMPLE_TWO_TAB_WORKSPACE).unwrap();

        save_to(
            dir.path(),
            &[Path::new("a.md"), Path::new("b.md"), Path::new("c.md")],
            None,
        )
        .unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let leaves = saved["main"]["children"][0]["children"].as_array().unwrap();
        let new_leaf = leaves
            .iter()
            .find(|leaf| leaf["state"]["state"]["file"] == "c.md")
            .unwrap();
        let id = new_leaf["id"].as_str().unwrap();

        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Nothing in the id format rules out a collision. Pins the assumption that one never
    /// happens in practice.
    #[test]
    fn new_leaf_id_is_distinct_across_many_calls_on_one_thread() {
        let ids: HashSet<String> = (0..1000).map(|_| new_leaf_id()).collect();
        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn save_to_sets_active_and_fronts_the_recent_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), WORKSPACE_WITH_SIDEBARS).unwrap();

        save_to(
            dir.path(),
            &[Path::new("notes/a.md"), Path::new("b.md")],
            Some(Path::new("b.md")),
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();
        assert_eq!(workspace.active_file(), Some(Path::new("b.md")));
        assert_eq!(
            workspace.last_open_files().first(),
            Some(&PathBuf::from("b.md"))
        );
        assert!(workspace
            .last_open_files()
            .contains(&PathBuf::from("image.png")));
    }

    #[test]
    fn save_to_replaces_all_closed_tabs_with_a_placeholder() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), SIMPLE_TWO_TAB_WORKSPACE).unwrap();

        save_to(dir.path(), &[], None).unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let leaves = saved["main"]["children"][0]["children"].as_array().unwrap();

        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0]["state"]["type"], "empty");
    }

    #[test]
    fn save_to_keeps_an_emptied_group_valid_across_repeated_saves() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), TWO_GROUP_WORKSPACE).unwrap();

        save_to(dir.path(), &[Path::new("a.md")], Some(Path::new("a.md"))).unwrap();
        save_to(dir.path(), &[Path::new("a.md")], Some(Path::new("a.md"))).unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let group_2 = saved["main"]["children"][1]["children"].as_array().unwrap();

        assert_eq!(group_2.len(), 1);
        assert_eq!(group_2[0]["state"]["type"], "empty");
    }

    #[test]
    fn save_to_round_trips_when_nothing_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), SIMPLE_TWO_TAB_WORKSPACE).unwrap();

        save_to(
            dir.path(),
            &[Path::new("a.md"), Path::new("b.md")],
            Some(Path::new("a.md")),
        )
        .unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
                .unwrap();
        let mut expected: Value = serde_json::from_str(SIMPLE_TWO_TAB_WORKSPACE).unwrap();
        expected["lastOpenFiles"] = json!(["a.md"]);

        assert_eq!(saved, expected);
    }

    #[test]
    fn save_to_creates_workspace_json_when_missing() {
        let dir = tempdir().unwrap();

        save_to(dir.path(), &[Path::new("a.md")], Some(Path::new("a.md"))).unwrap();

        let workspace = load_from(dir.path()).unwrap();
        assert_eq!(workspace.open_files(), [Path::new("a.md")]);
        assert_eq!(workspace.active_file(), Some(Path::new("a.md")));
    }

    #[test]
    fn save_to_does_nothing_without_an_obsidian_directory() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("not-a-vault/.obsidian");

        save_to(&missing, &[Path::new("a.md")], Some(Path::new("a.md"))).unwrap();

        assert!(!missing.exists());
    }
}
