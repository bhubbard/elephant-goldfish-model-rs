// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Error, Debug)]
pub enum PackagingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub const SKILLS: &[&str] = &[
    "egm-bootstrap",
    "egm-brainstorm",
    "egm-draft",
    "egm-drift-report",
    "egm-resolve",
    "egm-review",
    "egm-review-loop",
];

pub const PERSONAS: &[&str] = &[
    "bootstrap_orchestrator",
    "codebase_details",
    "comprehension",
    "design_critic",
    "implementation_readiness",
    "structural",
    "style",
    "synthesizer",
    "zoom_out",
];

pub const STANDARDS: &[&str] = &[
    "feature-spec",
    "heilmeier",
    "implementation-readiness",
    "re-implementation",
    "zoom-out",
];

pub const SHARED_PATTERNS: &[&str] = &[
    "plugin.json",
    "LICENSE",
    "README.md",
    "skills/*/SKILL.md",
    "skills/*/agents/openai.yaml",
    "skills/*/assets/*.svg",
    "skills/*/assets/*.png",
    "skills/*/references/*.md",
    "skills/*/references/*.json",
    "agents/*.md",
    "resources/*.md",
    "resources/document_standards/*.md",
    "docs/index.md",
    "docs/getting_started.md",
    "docs/harnesses.md",
    "docs/claude.md",
    "docs/codex.md",
    "docs/faq.md",
    "docs/privacy.md",
    "docs/releases.md",
    "assets/*.svg",
    "assets/*.png",
    "docs/assets/*.svg",
    "docs/assets/*.png",
];

pub fn host_patterns(target: &str) -> Result<Vec<&'static str>, PackagingError> {
    match target {
        "claude" => Ok(vec![
            ".claude-plugin/plugin.json",
            ".claude-plugin/icon.png",
            ".claude-plugin/agents/*.md",
        ]),
        "openai" => Ok(vec![".codex-plugin/plugin.json"]),
        _ => Err(PackagingError::Validation(format!(
            "Unsupported target: {}. Must be 'claude' or 'openai'",
            target
        ))),
    }
}

pub fn get_required_shared() -> Vec<String> {
    let mut req = vec![
        "plugin.json".to_string(),
        "LICENSE".to_string(),
        "README.md".to_string(),
        "assets/icon.svg".to_string(),
        "docs/index.md".to_string(),
        "docs/getting_started.md".to_string(),
        "docs/harnesses.md".to_string(),
        "docs/claude.md".to_string(),
        "docs/codex.md".to_string(),
        "docs/faq.md".to_string(),
        "docs/privacy.md".to_string(),
        "resources/README.md".to_string(),
        "resources/execution_instructions.md".to_string(),
        "skills/egm-bootstrap/references/worker-return.schema.json".to_string(),
        "skills/egm-bootstrap/references/templates.md".to_string(),
        "skills/egm-draft/references/design_doc.md".to_string(),
    ];

    for skill in SKILLS {
        req.push(format!("skills/{}/SKILL.md", skill));
        req.push(format!("skills/{}/agents/openai.yaml", skill));
        req.push(format!("skills/{}/assets/icon.svg", skill));
    }

    for persona in PERSONAS {
        req.push(format!("agents/{}.md", persona));
    }

    for std_name in STANDARDS {
        req.push(format!(
            "resources/document_standards/{}-standard.md",
            std_name
        ));
    }

    req
}

pub fn validate_version_for_tag<P: AsRef<Path>>(
    root: P,
    tag: &str,
) -> Result<String, PackagingError> {
    let tag_re = Regex::new(r"^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$").unwrap();
    if !tag_re.is_match(tag) {
        return Err(PackagingError::Validation(
            "Tag must be vMAJOR.MINOR.PATCH without leading zeroes".to_string(),
        ));
    }

    let version = &tag[1..];
    let root_path = root.as_ref();

    for relative in &[
        "plugin.json",
        ".claude-plugin/plugin.json",
        ".codex-plugin/plugin.json",
    ] {
        let manifest_path = root_path.join(relative);
        if !manifest_path.exists() {
            return Err(PackagingError::Validation(format!(
                "Manifest not found: {}",
                relative
            )));
        }
        let content = std::fs::read_to_string(&manifest_path)?;
        let val: serde_json::Value = serde_json::from_str(&content)?;
        let m_version = val.get("version").and_then(|v| v.as_str()).unwrap_or("");
        if m_version != version {
            return Err(PackagingError::Validation(format!(
                "{}: version must match {}",
                relative, tag
            )));
        }
    }

    Ok(version.to_string())
}

pub fn build_package<P: AsRef<Path>, Q: AsRef<Path>>(
    root: P,
    target: &str,
    output_dir: Q,
) -> Result<(PathBuf, PathBuf), PackagingError> {
    let root = root
        .as_ref()
        .canonicalize()
        .map_err(|e| PackagingError::Validation(format!("Cannot canonicalize root: {}", e)))?;
    let output_parent = output_dir.as_ref().to_path_buf();

    let folder = output_parent.join(format!("egm-{}", target));
    let archive = output_parent.join(format!("egm-{}.zip", target));

    if folder.exists() || archive.exists() {
        return Err(PackagingError::Validation(
            "Output folder or ZIP already exists".to_string(),
        ));
    }

    if folder == root || root.starts_with(&folder) {
        return Err(PackagingError::Validation(
            "Output must not contain the source tree".to_string(),
        ));
    }

    let host_pats = host_patterns(target)?;
    let mut files_to_include = BTreeSet::new();

    // Match shared patterns
    for pat in SHARED_PATTERNS.iter().chain(host_pats.iter()) {
        let glob_path = root.join(pat);
        let glob_str = glob_path.to_string_lossy();
        for entry in glob::glob(&glob_str)
            .map_err(|e| PackagingError::Validation(format!("Glob error {}: {}", pat, e)))?
        {
            let path = entry.map_err(|e| PackagingError::Validation(e.to_string()))?;
            let rel = path
                .strip_prefix(&root)
                .map_err(|e| PackagingError::Validation(e.to_string()))?
                .to_path_buf();

            // Check symlinks in components
            let mut curr = root.clone();
            for component in rel.iter() {
                curr.push(component);
                if curr.is_symlink() {
                    return Err(PackagingError::Validation(format!(
                        "Symlink in runtime path: {}",
                        rel.display()
                    )));
                }
            }

            let canonical = path
                .canonicalize()
                .map_err(|e| PackagingError::Validation(e.to_string()))?;
            if !canonical.starts_with(&root) {
                return Err(PackagingError::Validation(format!(
                    "Path outside source: {}",
                    rel.display()
                )));
            }

            if path.is_file() {
                files_to_include.insert(rel);
            }
        }
    }

    // Check required shared files
    let required = get_required_shared();
    let host_first = host_pats[0];
    for req in required
        .iter()
        .chain(std::iter::once(&host_first.to_string()))
    {
        let req_path = PathBuf::from(req);
        if !files_to_include.contains(&req_path) {
            return Err(PackagingError::Validation(format!(
                "Missing required file: {}",
                req
            )));
        }
    }

    // Validate manifests
    let host_manifest_path = root.join(host_pats[0]);
    let host_content = std::fs::read_to_string(&host_manifest_path)?;
    let host_manifest: serde_json::Value = serde_json::from_str(&host_content)?;

    let portable_manifest_path = root.join("plugin.json");
    let portable_content = std::fs::read_to_string(&portable_manifest_path)?;
    let portable_manifest: serde_json::Value = serde_json::from_str(&portable_content)?;

    let openai_meta = portable_manifest
        .get("extensions")
        .and_then(|e| e.get("com.openai"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // Reject directory packages with hooks, mcpServers, mcp, apps
    for metadata in &[&host_manifest, &portable_manifest, &openai_meta] {
        if let Some(hooks) = metadata.get("hooks") {
            if !hooks.is_null() && !hooks.as_object().is_some_and(|o| o.is_empty()) {
                return Err(PackagingError::Validation(
                    "Directory packages cannot contain hooks".to_string(),
                ));
            }
        }
        for key in &["mcpServers", "mcp", "apps"] {
            if let Some(v) = metadata.get(key) {
                if !v.is_null() && !v.as_object().is_some_and(|o| o.is_empty()) {
                    return Err(PackagingError::Validation(
                        "This runtime builder does not package MCP or app dependencies".to_string(),
                    ));
                }
            }
        }
    }

    if target == "openai" {
        let mut interfaces = Vec::new();
        if let Some(iface) = host_manifest.get("interface") {
            interfaces.push(iface);
        }
        if let Some(iface) = openai_meta.get("interface") {
            interfaces.push(iface);
        }
        for iface in interfaces {
            for key in &["composerIcon", "logo"] {
                if iface.get(key).and_then(|v| v.as_str()).is_none() {
                    return Err(PackagingError::Validation(format!(
                        "Missing OpenAI interface reference: {}",
                        key
                    )));
                }
            }
        }
    }

    // Create target directory structure
    std::fs::create_dir_all(&folder)?;

    // Create Zip archive
    let zip_file = File::create(&archive)?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for rel in files_to_include {
        let src_path = root.join(&rel);
        let dest_path = folder.join(&rel);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src_path, &dest_path)?;

        let mut f = File::open(&src_path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;

        let zip_path = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(zip_path, options)?;
        zip.write_all(&buffer)?;
    }

    zip.finish()?;

    Ok((folder, archive))
}

pub fn prepare_release<P: AsRef<Path>, Q: AsRef<Path>>(
    root: P,
    tag: &str,
    commit: &str,
    output_dir: Q,
) -> Result<PathBuf, PackagingError> {
    let version = validate_version_for_tag(&root, tag)?;

    let hex_re = Regex::new(r"^[0-9a-f]{40}$").unwrap();
    if !hex_re.is_match(commit) {
        return Err(PackagingError::Validation(
            "Source commit must be a full Git SHA (40 lowercase hex digits)".to_string(),
        ));
    }

    let output = output_dir.as_ref().to_path_buf();
    if output.exists() {
        return Err(PackagingError::Validation(format!(
            "Output destination already exists: {}",
            output.display()
        )));
    }

    std::fs::create_dir_all(&output)?;

    let build_temp = output.join("build");
    let mut checksum_lines = Vec::new();

    for target in &["claude", "openai"] {
        let target_build_dir = build_temp.join(target);
        std::fs::create_dir_all(&target_build_dir)?;

        let (_folder, archive) = build_package(&root, target, &target_build_dir)?;
        let destination = output.join(format!("egm-{}-{}.zip", target, version));
        std::fs::rename(&archive, &destination)?;

        let mut file_bytes = Vec::new();
        File::open(&destination)?.read_to_end(&mut file_bytes)?;
        let mut hasher = Sha256::new();
        hasher.update(&file_bytes);
        let digest = format!("{:x}", hasher.finalize());

        checksum_lines.push(format!(
            "{}  {}\n",
            digest,
            destination.file_name().unwrap().to_str().unwrap()
        ));
    }

    // Clean up temporary build directory
    let _ = std::fs::remove_dir_all(build_temp);

    // Write SHA256SUMS
    let sha256_path = output.join("SHA256SUMS");
    let mut sha_file = File::create(&sha256_path)?;
    sha_file.write_all(checksum_lines.concat().as_bytes())?;

    // Write RELEASE_NOTES.md
    let notes_path = output.join("RELEASE_NOTES.md");
    let notes_content = format!(
        r#"# EGM {version}

Declared source: `{commit}`
Tag: `{tag}`

Local source identity is caller-supplied and is not verified by this script. Publish only workflow-built candidates after exact-artifact checks.

Runtime archives: Claude and OpenAI. Verify downloads with `sha256sum -c SHA256SUMS`.

For CI-built candidates, verify each ZIP with `gh attestation verify FILE --repo google/elephant-goldfish-model --signer-workflow google/elephant-goldfish-model/.github/workflows/release.yml --source-ref refs/heads/main`. Provenance identifies the main-branch workflow run; the tagged source commit is recorded above. Local builds have no attestation.

## Changes

Maintainer: summarize changes and migration steps before publishing.

## Verification

Offline unit tests and runtime packaging are checked by CI. Live installation, workflow execution, reviewer isolation, directory approval, and cloud-host support are not established by this build. Maintainer: attach exact-bundle host evidence and limitations before publishing.
"#
    );
    let mut notes_file = File::create(&notes_path)?;
    notes_file.write_all(notes_content.as_bytes())?;

    Ok(output)
}
