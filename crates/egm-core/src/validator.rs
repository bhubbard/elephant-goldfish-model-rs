// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use crate::skills::{discover_skills, MAX_SKILL_BODY_BYTES};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Validation failures encountered:\n{0}")]
    Failures(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Failed(String),
    Warning(String),
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
}

#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    pub results: Vec<CheckResult>,
}

impl ValidationReport {
    pub fn is_success(&self) -> bool {
        self.results
            .iter()
            .all(|r| !matches!(r.status, CheckStatus::Failed(_)))
    }

    pub fn failures(&self) -> Vec<&CheckResult> {
        self.results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Failed(_)))
            .collect()
    }
}

pub struct Validator {
    root: PathBuf,
}

impl Validator {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn run_all_checks(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        report.results.push(self.check_manifest_sync());
        report.results.push(self.check_frontmatter());
        report.results.push(self.check_skill_load_limits());
        report.results.push(self.check_scenario_coverage());
        report.results.push(self.check_markdown_tables());
        report.results.push(self.check_link_integrity());
        report.results.push(self.check_style_persona());

        report
    }

    pub fn check_manifest_sync(&self) -> CheckResult {
        let name = "manifest_synchronization".to_string();
        let plugin_json_path = self.root.join("plugin.json");
        if !plugin_json_path.exists() {
            return CheckResult {
                name,
                status: CheckStatus::Failed("plugin.json is missing".to_string()),
            };
        }

        let main_json: serde_json::Value = match std::fs::read_to_string(&plugin_json_path)
            .and_then(|c| {
                serde_json::from_str(&c)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
            Ok(v) => v,
            Err(e) => {
                return CheckResult {
                    name,
                    status: CheckStatus::Failed(format!("Failed to read plugin.json: {}", e)),
                };
            }
        };

        let version = main_json
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if version.is_empty() {
            return CheckResult {
                name,
                status: CheckStatus::Failed("plugin.json is missing version".to_string()),
            };
        }

        let host_manifests = [".claude-plugin/plugin.json", ".codex-plugin/plugin.json"];

        let mut errors = Vec::new();
        for rel in &host_manifests {
            let path = self.root.join(rel);
            if !path.exists() {
                errors.push(format!("Missing manifest: {}", rel));
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let v = val.get("version").and_then(|x| x.as_str()).unwrap_or("");
                    if v != version {
                        errors.push(format!(
                            "{}: version '{}' does not match root version '{}'",
                            rel, v, version
                        ));
                    }
                } else {
                    errors.push(format!("Failed to parse JSON in {}", rel));
                }
            } else {
                errors.push(format!("Failed to read {}", rel));
            }
        }

        if errors.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(errors.join("\n")),
            }
        }
    }

    pub fn check_frontmatter(&self) -> CheckResult {
        let name = "skill_frontmatter".to_string();
        let skills_dir = self.root.join("skills");
        let skills = match discover_skills(&skills_dir) {
            Ok(s) => s,
            Err(e) => {
                return CheckResult {
                    name,
                    status: CheckStatus::Failed(e.to_string()),
                };
            }
        };

        if skills.is_empty() {
            return CheckResult {
                name,
                status: CheckStatus::Failed(
                    "No skills discovered in skills/ directory".to_string(),
                ),
            };
        }

        let mut errors = Vec::new();
        for skill in skills {
            if skill.frontmatter.name.trim().is_empty() {
                errors.push(format!("{}: empty 'name'", skill.skill_file.display()));
            }
            if skill.frontmatter.description.trim().is_empty() {
                errors.push(format!(
                    "{}: empty 'description'",
                    skill.skill_file.display()
                ));
            }
        }

        if errors.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(errors.join("\n")),
            }
        }
    }

    pub fn check_skill_load_limits(&self) -> CheckResult {
        let name = "skill_load_limits".to_string();
        let skills_dir = self.root.join("skills");
        let skills = match discover_skills(&skills_dir) {
            Ok(s) => s,
            Err(e) => {
                return CheckResult {
                    name,
                    status: CheckStatus::Failed(e.to_string()),
                };
            }
        };

        let mut errors = Vec::new();
        for skill in skills {
            if skill.body_bytes > MAX_SKILL_BODY_BYTES {
                errors.push(format!(
                    "{}: {} bytes exceeds Antigravity's {} byte limit",
                    skill.id, skill.body_bytes, MAX_SKILL_BODY_BYTES
                ));
            }
        }

        if errors.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(errors.join("\n")),
            }
        }
    }

    pub fn check_scenario_coverage(&self) -> CheckResult {
        let name = "scenario_coverage".to_string();
        let skills_dir = self.root.join("skills");
        if !skills_dir.exists() {
            return CheckResult {
                name,
                status: CheckStatus::Failed("skills/ directory not found".to_string()),
            };
        }

        let mut errors = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.join("SKILL.md").exists() {
                    let scenario_file = p.join("test_scenarios.yaml");
                    if !scenario_file.exists() {
                        errors.push(format!(
                            "Missing required test_scenarios.yaml in {}",
                            p.display()
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(errors.join("\n")),
            }
        }
    }

    pub fn check_markdown_tables(&self) -> CheckResult {
        let name = "markdown_tables".to_string();
        let fence_re = Regex::new(r"```[\s\S]*?```").unwrap();
        let continuation_re = Regex::new(r"^:.*:\s*$").unwrap();

        let mut failures = Vec::new();

        for entry in WalkDir::new(&self.root).into_iter().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let rel = path.strip_prefix(&self.root).unwrap_or(path);
                if rel.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s.starts_with('.') || s == "node_modules" || s == "target"
                }) {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(path) {
                    let sanitized = fence_re.replace_all(&content, |caps: &regex::Captures| {
                        "\n".repeat(caps[0].matches('\n').count())
                    });

                    let mut previous = String::new();
                    for (line_num, line) in sanitized.lines().enumerate() {
                        if continuation_re.is_match(line)
                            && (previous.starts_with('|') || previous.starts_with(':'))
                        {
                            failures.push(format!("{}:{}", rel.display(), line_num + 1));
                        }
                        previous = line.to_string();
                    }
                }
            }
        }

        if failures.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(format!(
                    "Multi-line table continuation rows found:\n{}",
                    failures.join("\n")
                )),
            }
        }
    }

    pub fn check_link_integrity(&self) -> CheckResult {
        let name = "link_integrity".to_string();
        let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        let fence_re = Regex::new(r"```[\s\S]*?```").unwrap();
        let inline_code_re = Regex::new(r"`[^`]*`").unwrap();
        let header_re = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();

        let mut failures = Vec::new();

        for entry in WalkDir::new(&self.root).into_iter().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let rel = path.strip_prefix(&self.root).unwrap_or(path);
                if rel.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s.starts_with('.') || s == "node_modules" || s == "target"
                }) {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(path) {
                    let headers = extract_slugs(&content, &header_re);

                    // Strip code blocks and inline code
                    let clean = fence_re.replace_all(&content, "");
                    let clean = inline_code_re.replace_all(&clean, "");

                    for cap in link_re.captures_iter(&clean) {
                        let target = cap[2].trim();

                        // Skip ignored schemes
                        if target.starts_with("http://")
                            || target.starts_with("https://")
                            || target.starts_with("mailto:")
                            || target.starts_with("file://")
                        {
                            continue;
                        }

                        // Ignore template variables or placeholders
                        if target.ends_with("_url")
                            || target.ends_with("_path")
                            || target.ends_with("_link")
                            || target.contains('{')
                            || target.contains('<')
                            || target.contains('[')
                        {
                            continue;
                        }

                        // Intra-document anchor link
                        if let Some(anchor) = target.strip_prefix('#') {
                            let anchor_lower = anchor.to_lowercase();
                            if !headers.is_empty() && !headers.contains(&anchor_lower) {
                                failures.push(format!(
                                    "{}: broken intra-document anchor '{}'",
                                    rel.display(),
                                    target
                                ));
                            }
                            continue;
                        }

                        // Relative file link
                        let file_part = target.split('#').next().unwrap_or(target);
                        if file_part.is_empty() {
                            continue;
                        }

                        let mut resolved = path.parent().unwrap_or(&self.root).join(file_part);
                        if !resolved.exists() && self.root.join(file_part).exists() {
                            resolved = self.root.join(file_part);
                        }

                        if !resolved.exists() {
                            failures.push(format!(
                                "{}: broken relative link '{}' -> resolved path '{}' not found",
                                rel.display(),
                                target,
                                resolved.display()
                            ));
                        }
                    }
                }
            }
        }

        if failures.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(failures.join("\n")),
            }
        }
    }

    pub fn check_style_persona(&self) -> CheckResult {
        let name = "style_persona_actions".to_string();
        let style_path = self.root.join("agents/style.md");
        if !style_path.exists() {
            return CheckResult {
                name,
                status: CheckStatus::Failed("agents/style.md is missing".to_string()),
            };
        }

        let content = match std::fs::read_to_string(&style_path) {
            Ok(c) => c,
            Err(e) => {
                return CheckResult {
                    name,
                    status: CheckStatus::Failed(format!("Failed to read agents/style.md: {}", e)),
                };
            }
        };

        let mut errors = Vec::new();
        for action in &["DROP", "CALIBRATE", "RELOCATE", "KEEP"] {
            let pat = format!("`{}`", action);
            if !content.contains(&pat) {
                errors.push(format!("Missing editorial calibration action `{}`", action));
            }
        }

        if !content.contains("Defensive Caveats & Uncalibrated Claims") {
            errors.push("Missing 'Defensive Caveats & Uncalibrated Claims' section".to_string());
        }

        if errors.is_empty() {
            CheckResult {
                name,
                status: CheckStatus::Passed,
            }
        } else {
            CheckResult {
                name,
                status: CheckStatus::Failed(errors.join("\n")),
            }
        }
    }
}

pub fn extract_slugs(content: &str, header_re: &Regex) -> HashSet<String> {
    let mut slugs = HashSet::new();
    let punct_re = Regex::new(r"[^a-z0-9 -]").unwrap();

    for cap in header_re.captures_iter(content) {
        let title = cap[2].trim().to_lowercase();
        let stripped = punct_re.replace_all(&title, "");
        let slug = stripped.replace(' ', "-");
        slugs.insert(slug);
    }
    slugs
}
