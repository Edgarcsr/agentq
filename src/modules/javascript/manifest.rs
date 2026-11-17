//! Dependencies declared by a Node project: `package.json` and the lockfiles of
//! npm, Yarn and pnpm.
//!
//! Having a package is not the same as calling it, so a dependency is
//! reported with lower confidence than a call found in the source. What a
//! package is for comes from `knowledge/packages.toml`. A package that is only
//! in a lockfile (a dependency of a dependency) is reported with the lowest
//! confidence, since the project may never reach the cryptography inside it.

use crate::finding::{Confidence, Finding, Purpose};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("../../knowledge/packages.toml");

/// Same id the source rules use for an algorithm the knowledge base lacks;
/// such findings are listed apart for manual review.
const UNRECOGNIZED: &str = "unrecognized";

/// Longest snippet kept in a finding, in characters.
const MAX_SNIPPET: usize = 120;

/// File names this module reads.
pub const FILES: &[&str] = &[
    "package.json",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
];

/// Sections of `package.json` that list dependencies, and how sure a package
/// there is to ship with the project.
const SECTIONS: &[(&str, Confidence)] = &[
    ("dependencies", Confidence::Medium),
    ("optionalDependencies", Confidence::Medium),
    ("peerDependencies", Confidence::Low),
    ("devDependencies", Confidence::Low),
];

#[derive(Deserialize)]
struct Table {
    package: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    /// The algorithm is chosen in code, so the dependency is only flagged.
    #[serde(default)]
    review: bool,
    #[serde(default)]
    uses: Vec<Use>,
}

#[derive(Deserialize)]
struct Use {
    algorithm: String,
    purpose: Purpose,
}

fn packages() -> &'static HashMap<String, Package> {
    static TABLE: OnceLock<HashMap<String, Package>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table: Table = toml::from_str(EMBEDDED).expect("packages.toml is valid");
        table
            .package
            .into_iter()
            .map(|package| (package.name.clone(), package))
            .collect()
    })
}

/// Whether `path` is a manifest or lockfile this module reads.
pub fn is_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| FILES.contains(&name))
}

/// Findings for the dependencies declared in the manifest at `path`.
pub fn analyze(path: &Path, source: &str) -> Vec<Finding> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match name {
        "package.json" => package_json(path, source),
        "package-lock.json" | "npm-shrinkwrap.json" => npm_lock(path, source),
        "yarn.lock" => yarn_lock(path, source),
        "pnpm-lock.yaml" => pnpm_lock(path, source),
        _ => vec![],
    }
}

/// Where a dependency was found, and how to show it.
struct Site<'a> {
    path: &'a Path,
    /// 0-based line.
    line: usize,
    /// 1-based column.
    column: usize,
    snippet: String,
    confidence: Confidence,
}

/// One finding per algorithm the package is known for.
fn findings_for(package: &str, site: &Site) -> Vec<Finding> {
    let Some(known) = packages().get(package) else {
        return vec![];
    };
    let finding = |algorithm: &str, purpose: Purpose| Finding {
        file: site.path.to_path_buf(),
        line: site.line + 1,
        column: site.column,
        snippet: site.snippet.chars().take(MAX_SNIPPET).collect(),
        library: package.to_string(),
        algorithm: algorithm.to_string(),
        purpose,
        key_size: None,
        confidence: site.confidence,
    };
    if known.review {
        return vec![finding(UNRECOGNIZED, Purpose::Unknown)];
    }
    known
        .uses
        .iter()
        .map(|used| finding(&used.algorithm, used.purpose))
        .collect()
}

/// The line and column where `"key"` appears, searching after `"after"` when
/// given. A missing key falls back to the start of the file.
fn locate(source: &str, after: Option<&str>, key: &str) -> (usize, usize) {
    let needle = format!("\"{key}\"");
    let header = after.map(|a| format!("\"{a}\""));
    let mut started = header.is_none();
    for (index, line) in source.lines().enumerate() {
        let mut from = 0;
        if !started {
            match header
                .as_deref()
                .and_then(|h| line.find(h).map(|p| p + h.len()))
            {
                Some(end) => {
                    started = true;
                    from = end;
                }
                None => continue,
            }
        }
        if let Some(position) = line[from..].find(&needle) {
            return (index, from + position + 1);
        }
    }
    (0, 1)
}

fn trimmed_line(source: &str, line: usize) -> String {
    source.lines().nth(line).unwrap_or("").trim().to_string()
}

/// Names a `package.json` lists directly, in any section.
fn direct_names(manifest: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return names;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return names;
    };
    for (section, _) in SECTIONS {
        if let Some(object) = json.get(*section).and_then(Value::as_object) {
            names.extend(object.keys().cloned());
        }
    }
    names
}

/// The `package.json` that sits next to a lockfile.
fn sibling_names(lockfile: &Path) -> HashSet<String> {
    direct_names(&lockfile.with_file_name("package.json"))
}

fn package_json(path: &Path, source: &str) -> Vec<Finding> {
    let Ok(json) = serde_json::from_str::<Value>(source) else {
        return vec![];
    };
    let mut findings = Vec::new();
    for (section, confidence) in SECTIONS {
        let Some(object) = json.get(*section).and_then(Value::as_object) else {
            continue;
        };
        for name in object.keys() {
            if !packages().contains_key(name.as_str()) {
                continue;
            }
            let (line, column) = locate(source, Some(section), name);
            let site = Site {
                path,
                line,
                column,
                snippet: trimmed_line(source, line),
                confidence: *confidence,
            };
            findings.extend(findings_for(name, &site));
        }
    }
    findings
}

/// A package that only a lockfile lists: nobody asked for it directly.
fn transitive(
    path: &Path,
    source: &str,
    name: &str,
    version: &str,
    line: usize,
    column: usize,
) -> Vec<Finding> {
    let shown = if version.is_empty() {
        name.to_string()
    } else {
        format!("{name}@{version}")
    };
    let site = Site {
        path,
        line,
        column,
        snippet: format!("{shown} (transitive dependency)"),
        confidence: Confidence::Low,
    };
    let _ = source;
    findings_for(name, &site)
}

fn npm_lock(path: &Path, source: &str) -> Vec<Finding> {
    let Ok(json) = serde_json::from_str::<Value>(source) else {
        return vec![];
    };
    let direct = sibling_names(path);
    let mut seen = HashSet::new();
    let mut findings = Vec::new();

    if let Some(entries) = json.get("packages").and_then(Value::as_object) {
        // lockfileVersion 2 and 3: "node_modules/a/node_modules/b": { "version": ... }
        for (key, entry) in entries {
            let name = key
                .rsplit_once("node_modules/")
                .map_or(key.as_str(), |(_, n)| n);
            if name.is_empty()
                || direct.contains(name)
                || !packages().contains_key(name)
                || !seen.insert(name.to_string())
            {
                continue;
            }
            let version = entry.get("version").and_then(Value::as_str).unwrap_or("");
            let (line, column) = locate(source, None, key);
            findings.extend(transitive(path, source, name, version, line, column));
        }
    } else if let Some(entries) = json.get("dependencies").and_then(Value::as_object) {
        // lockfileVersion 1: nested "dependencies" objects.
        fn walk(
            entries: &serde_json::Map<String, Value>,
            path: &Path,
            source: &str,
            direct: &HashSet<String>,
            seen: &mut HashSet<String>,
            out: &mut Vec<Finding>,
        ) {
            for (name, entry) in entries {
                if !direct.contains(name.as_str())
                    && packages().contains_key(name.as_str())
                    && seen.insert(name.clone())
                {
                    let version = entry.get("version").and_then(Value::as_str).unwrap_or("");
                    let (line, column) = locate(source, None, name);
                    out.extend(transitive(path, source, name, version, line, column));
                }
                if let Some(inner) = entry.get("dependencies").and_then(Value::as_object) {
                    walk(inner, path, source, direct, seen, out);
                }
            }
        }
        walk(entries, path, source, &direct, &mut seen, &mut findings);
    }
    findings
}

/// `name` of `name@range` or `@scope/name@range`.
fn package_of(spec: &str) -> Option<(&str, &str)> {
    let at = spec.rfind('@').filter(|at| *at > 0)?;
    Some((&spec[..at], &spec[at + 1..]))
}

fn yarn_lock(path: &Path, source: &str) -> Vec<Finding> {
    let direct = sibling_names(path);
    let lines: Vec<&str> = source.lines().collect();
    let mut seen = HashSet::new();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        // An entry header is flush left: `"a@^1", a@^1.2:`.
        if line.is_empty()
            || line.starts_with(char::is_whitespace)
            || line.starts_with('#')
            || !line.ends_with(':')
        {
            continue;
        }
        let header = line.trim_end_matches(':');
        let Some(name) = header
            .split(',')
            .filter_map(|spec| package_of(spec.trim().trim_matches('"')))
            .map(|(name, _)| name)
            .find(|name| packages().contains_key(*name))
        else {
            continue;
        };
        if direct.contains(name) || !seen.insert(name.to_string()) {
            continue;
        }
        let version = lines[index + 1..]
            .iter()
            .take_while(|l| !l.is_empty())
            .find_map(|l| l.trim().strip_prefix("version"))
            .map(|v| v.trim().trim_start_matches(':').trim().trim_matches('"'))
            .unwrap_or("");
        findings.extend(transitive(path, source, name, version, index, 1));
    }
    findings
}

fn pnpm_lock(path: &Path, source: &str) -> Vec<Finding> {
    let direct = sibling_names(path);
    let mut seen = HashSet::new();
    let mut findings = Vec::new();
    let mut in_packages = false;

    for (index, line) in source.lines().enumerate() {
        if !line.starts_with(char::is_whitespace) && !line.is_empty() {
            in_packages = line.trim_end() == "packages:";
            continue;
        }
        // Entries are indented by two spaces: `  /name@1.2.3:` or `  name@1.2.3:`.
        if !in_packages || !line.starts_with("  ") || line.starts_with("   ") {
            continue;
        }
        let key = line
            .trim()
            .trim_end_matches(':')
            .trim_matches('\'')
            .trim_matches('"')
            .trim_start_matches('/');
        // Drop peer-dependency suffixes such as `(react@18.2.0)`.
        let key = key.split('(').next().unwrap_or(key);
        let Some((name, version)) = package_of(key) else {
            continue;
        };
        if !packages().contains_key(name) || direct.contains(name) || !seen.insert(name.to_string())
        {
            continue;
        }
        findings.extend(transitive(path, source, name, version, index, 3));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn summary(findings: &[Finding]) -> Vec<(String, String, Purpose, Confidence, usize)> {
        findings
            .iter()
            .map(|f| {
                (
                    f.library.clone(),
                    f.algorithm.clone(),
                    f.purpose,
                    f.confidence,
                    f.line,
                )
            })
            .collect()
    }

    /// A folder with a `package.json`, for lockfiles that skip direct dependencies.
    fn project(manifest: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "agentq-manifest-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), manifest).unwrap();
        dir
    }

    #[test]
    fn the_embedded_table_parses_and_uses_known_algorithms() {
        let knowledge = crate::knowledge::KnowledgeBase::embedded();
        for package in packages().values() {
            assert!(
                package.review || !package.uses.is_empty(),
                "{}",
                package.name
            );
            for used in &package.uses {
                assert!(
                    knowledge.get(&used.algorithm).is_some(),
                    "{} names unknown algorithm {}",
                    package.name,
                    used.algorithm
                );
            }
        }
    }

    #[test]
    fn declared_dependencies_are_reported_where_they_are_written() {
        let source = r#"{
  "name": "app",
  "dependencies": {
    "express": "^4.0.0",
    "node-rsa": "^1.1.1"
  },
  "devDependencies": {
    "elliptic": "^6.5.4"
  }
}"#;
        let findings = analyze(Path::new("package.json"), source);
        assert_eq!(
            summary(&findings),
            vec![
                (
                    "node-rsa".into(),
                    "rsa".into(),
                    Purpose::Unknown,
                    Confidence::Medium,
                    5
                ),
                (
                    "elliptic".into(),
                    "ecdsa".into(),
                    Purpose::Signature,
                    Confidence::Low,
                    8
                ),
                (
                    "elliptic".into(),
                    "ecdh".into(),
                    Purpose::KeyExchange,
                    Confidence::Low,
                    8
                ),
            ]
        );
        assert_eq!(findings[0].snippet, "\"node-rsa\": \"^1.1.1\"");
        assert_eq!(findings[0].column, 5);
    }

    #[test]
    fn the_same_package_in_two_sections_is_found_in_each() {
        let source = r#"{"dependencies":{"elliptic":"1"},"devDependencies":{"elliptic":"1"}}"#;
        let findings = analyze(Path::new("package.json"), source);
        let confidences: Vec<_> = findings.iter().map(|f| f.confidence).collect();
        assert_eq!(
            confidences,
            vec![
                Confidence::Medium,
                Confidence::Medium,
                Confidence::Low,
                Confidence::Low
            ]
        );
        // Minified: both on line 1, in different places.
        assert!(findings[0].column < findings[2].column);
    }

    #[test]
    fn packages_that_take_the_algorithm_as_configuration_are_flagged_for_review() {
        let source = r#"{ "dependencies": { "jsonwebtoken": "^9.0.0" } }"#;
        let findings = analyze(Path::new("package.json"), source);
        assert_eq!(
            summary(&findings),
            vec![(
                "jsonwebtoken".into(),
                UNRECOGNIZED.into(),
                Purpose::Unknown,
                Confidence::Medium,
                1
            )]
        );
    }

    #[test]
    fn post_quantum_packages_are_listed_too() {
        let source = r#"{ "dependencies": { "@noble/post-quantum": "^0.2.0" } }"#;
        let algorithms: Vec<_> = analyze(Path::new("package.json"), source)
            .into_iter()
            .map(|f| f.algorithm)
            .collect();
        assert_eq!(algorithms, ["ml-kem", "ml-dsa", "slh-dsa"]);
    }

    #[test]
    fn unknown_packages_and_broken_json_give_nothing() {
        assert!(
            analyze(
                Path::new("package.json"),
                r#"{"dependencies":{"left-pad":"1"}}"#
            )
            .is_empty()
        );
        assert!(analyze(Path::new("package.json"), "{ not json").is_empty());
        assert!(analyze(Path::new("README.md"), "elliptic").is_empty());
    }

    #[test]
    fn an_npm_lockfile_reports_what_only_it_lists() {
        let dir = project(r#"{ "dependencies": { "node-rsa": "1" } }"#);
        let lock = r#"{
  "lockfileVersion": 3,
  "packages": {
    "": { "name": "app" },
    "node_modules/node-rsa": { "version": "1.1.1" },
    "node_modules/web3/node_modules/elliptic": { "version": "6.5.4" },
    "node_modules/elliptic": { "version": "6.6.1" }
  }
}"#;
        let findings = analyze(&dir.join("package-lock.json"), lock);
        // node-rsa is direct, so only its manifest entry reports it; elliptic is once.
        let found: Vec<_> = findings
            .iter()
            .map(|f| {
                (
                    f.library.as_str(),
                    f.algorithm.as_str(),
                    f.confidence,
                    f.line,
                )
            })
            .collect();
        assert_eq!(
            found,
            vec![
                ("elliptic", "ecdsa", Confidence::Low, 7),
                ("elliptic", "ecdh", Confidence::Low, 7),
            ]
        );
        // The hoisted copy (the one installed at the top) is the one reported.
        assert_eq!(
            findings[0].snippet,
            "elliptic@6.6.1 (transitive dependency)"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_old_npm_lockfile_with_nested_dependencies_is_read() {
        let dir = project("{}");
        let lock = r#"{
  "lockfileVersion": 1,
  "dependencies": {
    "web3": {
      "version": "1.0.0",
      "dependencies": {
        "secp256k1": { "version": "4.0.3" }
      }
    }
  }
}"#;
        let findings = analyze(&dir.join("package-lock.json"), lock);
        let libraries: Vec<_> = findings.iter().map(|f| f.library.as_str()).collect();
        assert_eq!(libraries, ["web3", "secp256k1", "secp256k1"]);
        assert_eq!(findings[1].line, 7);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_yarn_lockfile_is_read() {
        let dir = project("{}");
        let lock = "# yarn lockfile v1\n\n\
            \"@noble/curves@^1.0.0\", \"@noble/curves@^1.2.0\":\n  version \"1.2.0\"\n  resolved \"x\"\n\n\
            left-pad@^1.0.0:\n  version \"1.3.0\"\n";
        let findings = analyze(&dir.join("yarn.lock"), lock);
        assert_eq!(findings.len(), 4);
        assert!(findings.iter().all(|f| f.library == "@noble/curves"));
        assert_eq!(findings[0].line, 3);
        assert_eq!(
            findings[0].snippet,
            "@noble/curves@1.2.0 (transitive dependency)"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_pnpm_lockfile_is_read() {
        let dir = project("{}");
        let lock = "lockfileVersion: '9.0'\n\nimporters:\n  .:\n    dependencies:\n      elliptic:\n        version: 6.5.4\n\n\
            packages:\n\n  elliptic@6.5.4:\n    resolution: {integrity: sha512-x}\n\n  \
            '@noble/curves@1.2.0(typescript@5.0.0)':\n    resolution: {integrity: sha512-y}\n\nsnapshots:\n\n  node-rsa@1.1.1:\n    dependencies: {}\n";
        let findings = analyze(&dir.join("pnpm-lock.yaml"), lock);
        let libraries: Vec<_> = findings.iter().map(|f| f.library.as_str()).collect();
        // `snapshots:` is not read, and the importers' own entry is not a package.
        assert_eq!(
            libraries,
            [
                "elliptic",
                "elliptic",
                "@noble/curves",
                "@noble/curves",
                "@noble/curves",
                "@noble/curves"
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
