// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectEcosystem {
    Rust,
    Node,
    Python,
    Go,
    Cpp,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ProjectInspection {
    pub ecosystem: ProjectEcosystem,
    pub test_command: String,
    pub build_command: String,
    pub source_dirs: Vec<String>,
}

pub struct Bootstrapper;

impl Bootstrapper {
    pub fn inspect<P: AsRef<Path>>(root: P) -> ProjectInspection {
        let r = root.as_ref();

        if r.join("Cargo.toml").exists() {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Rust,
                test_command: "cargo test".to_string(),
                build_command: "cargo build".to_string(),
                source_dirs: vec!["src".to_string(), "crates".to_string()],
            }
        } else if r.join("package.json").exists() {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Node,
                test_command: "npm test".to_string(),
                build_command: "npm run build".to_string(),
                source_dirs: vec!["src".to_string(), "lib".to_string()],
            }
        } else if r.join("pyproject.toml").exists() || r.join("setup.py").exists() {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Python,
                test_command: "pytest".to_string(),
                build_command: "python -m build".to_string(),
                source_dirs: vec!["src".to_string(), "tests".to_string()],
            }
        } else if r.join("go.mod").exists() {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Go,
                test_command: "go test ./...".to_string(),
                build_command: "go build ./...".to_string(),
                source_dirs: vec!["cmd".to_string(), "pkg".to_string(), "internal".to_string()],
            }
        } else if r.join("CMakeLists.txt").exists() {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Cpp,
                test_command: "ctest".to_string(),
                build_command: "cmake --build build".to_string(),
                source_dirs: vec!["src".to_string(), "include".to_string()],
            }
        } else {
            ProjectInspection {
                ecosystem: ProjectEcosystem::Unknown,
                test_command: "make test".to_string(),
                build_command: "make".to_string(),
                source_dirs: vec!["src".to_string()],
            }
        }
    }

    pub fn bootstrap_docs<P: AsRef<Path>>(
        root: P,
        project_name: &str,
    ) -> Result<PathBuf, BootstrapError> {
        let r = root.as_ref();
        let designs_dir = r.join("docs").join("designs");
        std::fs::create_dir_all(&designs_dir)?;

        let inspection = Self::inspect(r);

        let arch_file = designs_dir.join("architecture.md");
        if !arch_file.exists() {
            let content = format!(
                r#"# {project_name} System Architecture

* **Status:** Current
* **Ecosystem:** {:?}
* **Verification Command:** `{}`

---

## 1. Overview
High-level overview of the {project_name} system, its purpose, and architectural goals.

## 2. Component Model
Key subsystems and packages:
{}

## 3. Verification & Development
* **Build:** `{}`
* **Test:** `{}`

## 4. Invariants & Living Contracts
Core safety boundaries, invariants, and guidelines for AI coding agents.
"#,
                inspection.ecosystem,
                inspection.test_command,
                inspection
                    .source_dirs
                    .iter()
                    .map(|d| format!("* `{}/`: Core subsystem directory", d))
                    .collect::<Vec<_>>()
                    .join("\n"),
                inspection.build_command,
                inspection.test_command
            );
            std::fs::write(&arch_file, content)?;
        }

        Ok(designs_dir)
    }
}
