#![cfg(test)]
//! Packaging of the agent plugins under `plugins/`. Each harness installs a plugin as one
//! self-contained folder, so the universal skills are copied into every harness plugin; these
//! tests keep the copies identical, the descriptions within harness limits, and every path a
//! manifest or marketplace names present.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The longest skill or agent description any supported harness shows in full.
const DESCRIPTION_LIMIT: usize = 200;

const UNIVERSAL: &str = "reprobit";

pub(crate) fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn plugins() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(repo().join("plugins"))
        .expect("plugins/ exists")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

pub(crate) fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| entry.expect("readable entry").path())
        .collect();
    paths.sort();
    paths
}

/// Every file below `dir`, relative to it.
pub(crate) fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for path in children(&next) {
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path.strip_prefix(dir).expect("below dir").to_path_buf());
            }
        }
    }
    found.sort();
    found
}

/// The value of `key` in a Markdown file's YAML frontmatter, joining a folded (`>-`) value.
pub(crate) fn frontmatter(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).expect("readable Markdown");
    let body = text.strip_prefix("---\n")?;
    let head = &body[..body.find("\n---")?];
    let mut lines = head.lines();
    let prefix = format!("{key}:");
    let first = lines.by_ref().find(|line| line.starts_with(&prefix))?;
    let value = first[prefix.len()..].trim();
    if !value.starts_with('>') && !value.starts_with('|') {
        return Some(value.trim_matches('"').to_owned());
    }
    let folded: Vec<&str> = lines
        .take_while(|line| line.starts_with(' '))
        .map(str::trim)
        .collect();
    Some(folded.join(" "))
}

fn check_description(owner: &Path, description: Option<String>, problems: &mut Vec<String>) {
    match description {
        None => problems.push(format!("{}: no description", owner.display())),
        Some(text) if text.chars().count() > DESCRIPTION_LIMIT => problems.push(format!(
            "{}: description is {} chars, over {DESCRIPTION_LIMIT}",
            owner.display(),
            text.chars().count()
        )),
        Some(_) => {}
    }
}

fn assert_none(problems: &[String]) {
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn skills_are_named_for_their_directory_and_described_briefly() {
    let mut problems = Vec::new();
    let mut count = 0;
    for plugin in plugins() {
        for skill in children(&plugin.join("skills")) {
            let file = skill.join("SKILL.md");
            count += 1;
            let dir = skill
                .file_name()
                .expect("named")
                .to_string_lossy()
                .into_owned();
            if frontmatter(&file, "name").as_deref() != Some(dir.as_str()) {
                problems.push(format!("{}: name is not {dir}", file.display()));
            }
            check_description(&file, frontmatter(&file, "description"), &mut problems);
        }
    }
    assert!(count > 0, "no skills found");
    assert_none(&problems);
}

#[test]
fn agents_are_described_briefly() {
    let mut problems = Vec::new();
    let mut dirs: Vec<PathBuf> = plugins()
        .iter()
        .map(|plugin| plugin.join("agents"))
        .collect();
    dirs.push(repo().join(".codex/agents"));
    let mut count = 0;
    for agent in dirs.iter().flat_map(|dir| children(dir)) {
        count += 1;
        match agent.extension().and_then(|ext| ext.to_str()) {
            Some("md") => {
                if frontmatter(&agent, "name").is_none() {
                    problems.push(format!("{}: no name", agent.display()));
                }
                check_description(&agent, frontmatter(&agent, "description"), &mut problems);
            }
            Some("toml") => {
                let text = fs::read_to_string(&agent).expect("readable TOML");
                let table: toml::Table = toml::from_str(&text).expect("valid TOML");
                let description = table
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
                check_description(&agent, description, &mut problems);
            }
            _ => problems.push(format!("{}: not an agent file", agent.display())),
        }
    }
    assert!(count > 0, "no agents found");
    assert_none(&problems);
}

#[test]
fn harness_plugins_carry_identical_copies_of_the_universal_skills() {
    let universal = repo().join("plugins").join(UNIVERSAL).join("skills");
    let mut problems = Vec::new();
    let harnesses: Vec<PathBuf> = plugins()
        .into_iter()
        .filter(|plugin| !plugin.ends_with(UNIVERSAL))
        .collect();
    assert!(!harnesses.is_empty(), "no harness plugins found");
    for plugin in harnesses {
        for skill in children(&universal) {
            let name = skill.file_name().expect("named");
            let copy = plugin.join("skills").join(name);
            let (want, got) = (files(&skill), files(&copy));
            if want != got {
                problems.push(format!(
                    "{}: files differ from {}",
                    copy.display(),
                    skill.display()
                ));
                continue;
            }
            for file in want {
                if fs::read(skill.join(&file)).ok() != fs::read(copy.join(&file)).ok() {
                    problems.push(format!(
                        "{}: differs; run `just plugins-sync`",
                        copy.join(&file).display()
                    ));
                }
            }
        }
    }
    assert_none(&problems);
}

#[test]
fn codex_agents_in_the_repo_match_the_plugin_copies() {
    let shipped = repo().join("plugins/reprobit-codex/agents");
    let local = repo().join(".codex/agents");
    assert_eq!(files(&shipped), files(&local));
    for file in files(&shipped) {
        assert_eq!(
            fs::read(shipped.join(&file)).ok(),
            fs::read(local.join(&file)).ok(),
            "{}",
            file.display()
        );
    }
}

/// Paths a manifest names: strings that start with `./`, and the `logo` and `icon` fields.
fn named_paths(value: &Value, key: Option<&str>, found: &mut Vec<String>) {
    match value {
        Value::String(text) if text.starts_with("./") || matches!(key, Some("logo" | "icon")) => {
            found.push(text.clone());
        }
        Value::Array(items) => items.iter().for_each(|item| named_paths(item, key, found)),
        Value::Object(map) => map.iter().for_each(|(k, v)| named_paths(v, Some(k), found)),
        _ => {}
    }
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("readable JSON")).expect("valid JSON")
}

#[test]
fn manifests_name_only_paths_that_exist() {
    const MANIFESTS: [&str; 4] = [
        ".claude-plugin/plugin.json",
        ".codex-plugin/plugin.json",
        "plugin.json",
        "package.json",
    ];
    let mut problems = Vec::new();
    let mut count = 0;
    for plugin in plugins() {
        for manifest in MANIFESTS
            .iter()
            .map(|name| plugin.join(name))
            .filter(|path| path.is_file())
        {
            count += 1;
            let mut paths = Vec::new();
            named_paths(&json(&manifest), None, &mut paths);
            for path in paths {
                let literal = path
                    .split('*')
                    .next()
                    .expect("one part")
                    .trim_end_matches('/');
                if !plugin.join(literal).exists() {
                    problems.push(format!("{}: {path} does not exist", manifest.display()));
                }
            }
        }
    }
    assert!(count >= 5, "found {count} manifests");
    assert_none(&problems);
}

#[test]
fn marketplaces_list_plugins_that_exist() {
    let claude = json(&repo().join(".claude-plugin/marketplace.json"));
    let codex = json(&repo().join(".agents/plugins/marketplace.json"));
    let mut problems = Vec::new();
    let entries = [
        (&claude, "/source", ".claude-plugin/plugin.json"),
        (&codex, "/source/path", ".codex-plugin/plugin.json"),
    ];
    for (market, pointer, manifest) in entries {
        let plugins = market["plugins"].as_array().expect("plugins array");
        assert!(!plugins.is_empty(), "empty marketplace");
        for plugin in plugins {
            let source = plugin
                .pointer(pointer)
                .and_then(Value::as_str)
                .expect("local source");
            let path = repo().join(source).join(manifest);
            if !path.is_file() {
                problems.push(format!("{source}: no {manifest}"));
                continue;
            }
            if json(&path)["name"] != plugin["name"] {
                problems.push(format!(
                    "{source}: manifest name differs from {}",
                    plugin["name"]
                ));
            }
        }
    }
    assert_none(&problems);
}
