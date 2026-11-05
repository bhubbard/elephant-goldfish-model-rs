// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use egm_core::{
    build_package, discover_skills, prepare_release, validate_version_for_tag, Bootstrapper,
    DocumentStandard, DraftGenerator, DriftAnalyzer, FindingSeverity, ProjectEcosystem,
    ReviewEngine, StandardType, Validator, MAX_SKILL_BODY_BYTES,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn test_validator_all_checks_pass() {
    let root = repo_root();
    let validator = Validator::new(&root);
    let report = validator.run_all_checks();

    assert!(
        report.is_success(),
        "Validation checks failed: {:?}",
        report.failures()
    );
}

#[test]
fn test_skills_fit_within_body_limits() {
    let root = repo_root();
    let skills = discover_skills(root.join("skills")).expect("Failed to discover skills");
    assert!(!skills.is_empty(), "No skills found");

    for skill in &skills {
        assert!(
            skill.is_within_size_limit(),
            "Skill {} exceeds max body limit ({} > {})",
            skill.id,
            skill.body_bytes,
            MAX_SKILL_BODY_BYTES
        );
        assert!(!skill.frontmatter.name.is_empty());
        assert!(!skill.frontmatter.description.is_empty());
    }
}

#[test]
fn test_standards_loading() {
    let root = repo_root();
    let standards = DocumentStandard::load_from_dir(root.join("resources/document_standards"))
        .expect("Failed to load standards");
    assert_eq!(standards.len(), 5);
}

#[test]
fn test_tag_validation() {
    let root = repo_root();
    // Valid tag matching manifests
    let version = validate_version_for_tag(&root, "v0.1.0").expect("Valid tag");
    assert_eq!(version, "0.1.0");

    // Invalid tags
    assert!(validate_version_for_tag(&root, "0.1.0").is_err());
    assert!(validate_version_for_tag(&root, "v01.1.0").is_err());
    assert!(validate_version_for_tag(&root, "v0.1.0-rc.1").is_err());
    assert!(validate_version_for_tag(&root, "v0.1.0;echo bad").is_err());
}

#[test]
fn test_packaging_claude_and_openai() {
    let root = repo_root();
    let temp_dir = TempDir::new().unwrap();

    let (claude_folder, claude_zip) =
        build_package(&root, "claude", temp_dir.path()).expect("Build claude");
    assert!(claude_folder.exists());
    assert!(claude_zip.exists());

    let (openai_folder, openai_zip) =
        build_package(&root, "openai", temp_dir.path()).expect("Build openai");
    assert!(openai_folder.exists());
    assert!(openai_zip.exists());
}

#[test]
fn test_prepare_release_flow() {
    let root = repo_root();
    let temp_dir = TempDir::new().unwrap();
    let release_dest = temp_dir.path().join("release_out");

    let fake_sha = "0123456789abcdef0123456789abcdef01234567";
    prepare_release(&root, "v0.1.0", fake_sha, &release_dest).expect("Prepare release");

    assert!(release_dest.join("egm-claude-0.1.0.zip").exists());
    assert!(release_dest.join("egm-openai-0.1.0.zip").exists());
    assert!(release_dest.join("SHA256SUMS").exists());
    assert!(release_dest.join("RELEASE_NOTES.md").exists());
}

#[test]
fn test_review_engine_detection() {
    let spec = r#"# Feature Spec: Webhook Delivery

## Overview & Goals
This is a problem statement describing our goals.

## Architecture & Design
Trade-offs and alternatives were considered carefully.
Failure modes and risks are handled via retry queues.

## Verification Plan
Automated tests and unit verification suite.
Rollback is supported by database migrations.
"#;
    let report = ReviewEngine::review_content("test.md".to_string(), spec).unwrap();
    assert!(report.approved);
}

#[test]
fn test_review_engine_rejects_incomplete_spec() {
    let spec = "# Stub Section\nBrief notes without objectives or validation.";
    let report = ReviewEngine::review_content("test.md".to_string(), spec).unwrap();
    assert!(!report.approved);
    assert!(report
        .findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Critical));
}

#[test]
fn test_draft_scaffolding() {
    let content = DraftGenerator::scaffold_spec("Auth Engine", StandardType::FeatureSpec, "Alice");
    assert!(content.contains("Feature Spec: Auth Engine"));
    assert!(content.contains("Alice"));
    assert!(content.contains("Context & Objectives"));
}

#[test]
fn test_bootstrapper_ecosystem_detection() {
    let root = repo_root();
    let inspection = Bootstrapper::inspect(&root);
    assert_eq!(inspection.ecosystem, ProjectEcosystem::Rust);
    assert_eq!(inspection.test_command, "cargo test");
}

#[test]
fn test_drift_analyzer_runs() {
    let root = repo_root();
    let analyzer = DriftAnalyzer::new(&root);
    let report = analyzer.scan_drift("docs/designs").expect("Scan drift");
    assert!(report.scanned_specs > 0);
}
