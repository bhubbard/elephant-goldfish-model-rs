// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum DriftError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftItem {
    pub spec_file: String,
    pub line_number: usize,
    pub referenced_path: String,
    pub issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub scanned_specs: usize,
    pub total_references: usize,
    pub missing_paths: Vec<DriftItem>,
    pub status: String,
}

impl DriftReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# EGM Living Documentation Drift Report\n\n");
        md.push_str(&format!(
            "* **Scanned Design Specs:** {}\n",
            self.scanned_specs
        ));
        md.push_str(&format!(
            "* **Total References Checked:** {}\n",
            self.total_references
        ));
        md.push_str(&format!(
            "* **Drift Issues Found:** {}\n\n",
            self.missing_paths.len()
        ));

        if self.missing_paths.is_empty() {
            md.push_str("> [!NOTE]\n> Zero documentation drift detected! All referenced source files and contracts exist on disk.\n");
            return md;
        }

        md.push_str("| Spec Location | Referenced Path | Drift Issue |\n| :--- | :--- | :--- |\n");
        for item in &self.missing_paths {
            md.push_str(&format!(
                "| `{}:{}` | `{}` | {} |\n",
                item.spec_file, item.line_number, item.referenced_path, item.issue
            ));
        }

        md
    }
}

pub struct DriftAnalyzer {
    repo_root: PathBuf,
}

impl DriftAnalyzer {
    pub fn new<P: AsRef<Path>>(repo_root: P) -> Self {
        Self {
            repo_root: repo_root.as_ref().to_path_buf(),
        }
    }

    pub fn scan_drift<P: AsRef<Path>>(&self, specs_dir: P) -> Result<DriftReport, DriftError> {
        let specs_path = self.repo_root.join(specs_dir.as_ref());
        let mut missing_paths = Vec::new();
        let mut scanned_specs = 0;
        let mut total_references = 0;

        if !specs_path.exists() {
            return Ok(DriftReport {
                scanned_specs: 0,
                total_references: 0,
                missing_paths: Vec::new(),
                status: "Specs directory does not exist yet".to_string(),
            });
        }

        let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        let code_path_re = Regex::new(r"`([a-zA-Z0-9_\-./]+\.[a-zA-Z0-9_]+)`").unwrap();

        for entry in WalkDir::new(&specs_path).into_iter().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                scanned_specs += 1;
                let rel_spec = path
                    .strip_prefix(&self.repo_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = std::fs::read_to_string(path) {
                    for (line_idx, line) in content.lines().enumerate() {
                        // Check markdown links
                        for cap in link_re.captures_iter(line) {
                            let target = &cap[2];
                            if target.starts_with("http")
                                || target.starts_with('#')
                                || target.starts_with("mailto")
                            {
                                continue;
                            }
                            total_references += 1;
                            let file_part = target.split('#').next().unwrap_or(target);
                            if !file_part.is_empty() {
                                let resolved =
                                    path.parent().unwrap_or(&self.repo_root).join(file_part);
                                if !resolved.exists() {
                                    missing_paths.push(DriftItem {
                                        spec_file: rel_spec.clone(),
                                        line_number: line_idx + 1,
                                        referenced_path: target.to_string(),
                                        issue: "Linked file does not exist on disk".to_string(),
                                    });
                                }
                            }
                        }

                        // Check inline code file paths
                        for cap in code_path_re.captures_iter(line) {
                            let candidate = &cap[1];
                            if candidate.contains('/') && !candidate.starts_with("http") {
                                total_references += 1;
                                let candidate_path = self.repo_root.join(candidate);
                                if !candidate_path.exists() {
                                    missing_paths.push(DriftItem {
                                        spec_file: rel_spec.clone(),
                                        line_number: line_idx + 1,
                                        referenced_path: candidate.to_string(),
                                        issue: "Referenced source file missing".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = if missing_paths.is_empty() {
            "Synchronized".to_string()
        } else {
            format!("Drift detected: {} broken references", missing_paths.len())
        };

        Ok(DriftReport {
            scanned_specs,
            total_references,
            missing_paths,
            status,
        })
    }
}
