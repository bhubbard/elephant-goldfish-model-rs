// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_SKILL_BODY_BYTES: usize = 24_000;

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Failed to read skill at {0}: {1}")]
    IoError(String, #[source] std::io::Error),
    #[error("Missing frontmatter delimiter in {0}")]
    MissingFrontmatter(String),
    #[error("Failed to parse YAML frontmatter in {0}: {1}")]
    YamlError(String, #[source] serde_yaml::Error),
    #[error("Skill validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(rename = "disable-model-invocation", default)]
    pub disable_model_invocation: Option<bool>,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub dir_path: PathBuf,
    pub skill_file: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub body: String,
    pub body_bytes: usize,
}

impl Skill {
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, SkillError> {
        let dir_path = dir.as_ref().to_path_buf();
        let skill_file = dir_path.join("SKILL.md");
        if !skill_file.exists() {
            return Err(SkillError::ValidationError(format!(
                "SKILL.md not found in {}",
                dir_path.display()
            )));
        }

        let content = std::fs::read_to_string(&skill_file)
            .map_err(|e| SkillError::IoError(skill_file.display().to_string(), e))?;

        let (fm, body) = extract_frontmatter_and_body(&content, &skill_file)?;
        let body_trimmed = body.trim();
        let body_bytes = body_trimmed.len();

        let id = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Ok(Skill {
            id,
            dir_path,
            skill_file,
            frontmatter: fm,
            body: body_trimmed.to_string(),
            body_bytes,
        })
    }

    pub fn is_within_size_limit(&self) -> bool {
        self.body_bytes <= MAX_SKILL_BODY_BYTES
    }
}

pub fn extract_frontmatter_and_body(
    content: &str,
    path: &Path,
) -> Result<(SkillFrontmatter, String), SkillError> {
    let lines: Vec<&str> = content.lines().collect();

    let mut start_idx = None;
    for (idx, line) in lines.iter().enumerate().take(15) {
        if line.trim() == "---" {
            start_idx = Some(idx);
            break;
        }
    }

    let start = start_idx.ok_or_else(|| {
        SkillError::MissingFrontmatter(format!(
            "No leading '---' frontmatter delimiter found in {}",
            path.display()
        ))
    })?;

    let mut end_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim() == "---" {
            end_idx = Some(idx);
            break;
        }
    }

    let end = end_idx.ok_or_else(|| {
        SkillError::MissingFrontmatter(format!(
            "No closing '---' frontmatter delimiter found in {}",
            path.display()
        ))
    })?;

    let yaml_slice = &lines[start + 1..end];
    let yaml_text = yaml_slice.join("\n");
    let fm: SkillFrontmatter = serde_yaml::from_str(&yaml_text)
        .map_err(|e| SkillError::YamlError(format!("in {}: {}", path.display(), e), e))?;

    if fm.name.trim().is_empty() {
        return Err(SkillError::ValidationError(format!(
            "'name' must be a non-empty string in {}",
            path.display()
        )));
    }
    if fm.description.trim().is_empty() {
        return Err(SkillError::ValidationError(format!(
            "'description' must be a non-empty string in {}",
            path.display()
        )));
    }

    let body_slice = &lines[end + 1..];
    let body = body_slice.join("\n");

    Ok((fm, body))
}

pub fn discover_skills<P: AsRef<Path>>(skills_dir: P) -> Result<Vec<Skill>, SkillError> {
    let p = skills_dir.as_ref();
    if !p.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();
    let entries =
        std::fs::read_dir(p).map_err(|e| SkillError::IoError(p.display().to_string(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| SkillError::IoError(p.display().to_string(), e))?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            skills.push(Skill::load_from_dir(&path)?);
        }
    }

    skills.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(skills)
}
