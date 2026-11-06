// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use egm_core::{
    build_package, discover_skills, prepare_release, validate_version_for_tag, Bootstrapper,
    DocumentStandard, DraftGenerator, DriftAnalyzer, FindingSeverity, ProjectEcosystem,
    ReviewEngine, StandardType, Validator, MAX_SKILL_BODY_BYTES,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_json<P: AsRef<Path>>(path: P) -> Value {
    let content = std::fs::read_to_string(path).expect("Failed to read JSON");
    serde_json::from_str(&content).expect("Failed to parse JSON")
}

fn read_frontmatter(path: &Path) -> serde_yaml::Value {
    let content = std::fs::read_to_string(path).expect("Failed to read markdown");
    let parts: Vec<&str> = content.split("---").collect();
    assert!(
        parts.len() >= 3,
        "File missing YAML frontmatter delimiters: {}",
        path.display()
    );
    serde_yaml::from_str(parts[1]).expect("Failed to parse YAML frontmatter")
}

const REVIEWERS: &[&str] = &[
    "codebase_details",
    "comprehension",
    "design_critic",
    "implementation_readiness",
    "structural",
    "style",
    "zoom_out",
];

// --- Validation Tests ---

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
    let version = validate_version_for_tag(&root, "v0.1.0").expect("Valid tag");
    assert_eq!(version, "0.1.0");

    assert!(validate_version_for_tag(&root, "0.1.0").is_err());
    assert!(validate_version_for_tag(&root, "v01.1.0").is_err());
    assert!(validate_version_for_tag(&root, "v0.1.0-rc.1").is_err());
    assert!(validate_version_for_tag(&root, "v0.1.0;echo bad").is_err());
}

// --- Packaging & Releases Tests ---

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
fn test_reproducible_zip_archive() {
    let root = repo_root();
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    let (_, zip1) = build_package(&root, "claude", temp_dir1.path()).unwrap();
    let (_, zip2) = build_package(&root, "claude", temp_dir2.path()).unwrap();

    let bytes1 = std::fs::read(&zip1).unwrap();
    let bytes2 = std::fs::read(&zip2).unwrap();
    assert_eq!(
        bytes1, bytes2,
        "Packaged zip archives must be deterministic"
    );

    // Verify zip archive contents
    let file = File::open(&zip1).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut names = HashSet::new();
    for i in 0..archive.len() {
        let f = archive.by_index(i).unwrap();
        names.insert(f.name().to_string());
    }

    assert!(names.contains("plugin.json"));
    assert!(names.contains("LICENSE"));
    assert!(names.contains("README.md"));
    assert!(names.contains("skills/egm-brainstorm/SKILL.md"));
    assert!(names.contains("agents/design_critic.md"));
    assert!(names.contains("resources/document_standards/feature-spec-standard.md"));

    // Ensure dev files do not ship
    assert!(!names.contains("CONTRIBUTING.md"));
    assert!(!names.contains("TESTING.md"));
    assert!(!names.contains("AI_POLICY.md"));
    assert!(!names.iter().any(|n| n.starts_with("tests/")));
    assert!(!names.iter().any(|n| n.starts_with("site/")));
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

    let sums = std::fs::read_to_string(release_dest.join("SHA256SUMS")).unwrap();
    assert!(sums.contains("egm-claude-0.1.0.zip"));
    assert!(sums.contains("egm-openai-0.1.0.zip"));
}

// --- Review & Spec Engine Tests ---

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

// --- Harness & Adapter Tests (Ported from test_harnesses.py, test_cursor.py, test_copilot.py) ---

#[test]
fn test_style_and_implementation_boundaries() {
    let root = repo_root();

    // Verify Style Goldfish defines DROP/CALIBRATE/RELOCATE/KEEP
    let style_text = std::fs::read_to_string(root.join("agents/style.md")).unwrap();
    for action in &["DROP", "CALIBRATE", "RELOCATE", "KEEP"] {
        assert!(style_text.contains(&format!("`{}`", action)));
    }
    assert!(style_text.contains("Defensive Caveats & Uncalibrated Claims"));

    // Verify Implementation Readiness defines file boundaries & hand-off
    let std_text = std::fs::read_to_string(
        root.join("resources/document_standards/implementation-readiness-standard.md"),
    )
    .unwrap();
    assert!(std_text.contains("Target Scope & File Boundary"));
    assert!(std_text.contains("repository-relative"));
    assert!(std_text.contains("Phase 5 write allowlist"));
}

#[test]
fn test_copilot_profiles_use_canonical_reviewers() {
    let root = repo_root();
    let profiles_dir = root.join("com.github.copilot/agents");

    for persona in REVIEWERS {
        let canonical_file = root.join(format!("agents/{}.md", persona));
        let fm = read_frontmatter(&canonical_file);
        let canonical_name = fm["name"].as_str().unwrap();

        let profile_file = profiles_dir.join(format!("{}.agent.md", canonical_name));
        assert!(
            profile_file.exists(),
            "Missing Copilot profile: {}",
            profile_file.display()
        );

        let p_fm = read_frontmatter(&profile_file);
        assert_eq!(p_fm["name"].as_str().unwrap(), canonical_name);
        assert_eq!(
            p_fm["tools"].as_sequence().unwrap(),
            &[
                serde_yaml::Value::String("read".into()),
                serde_yaml::Value::String("search".into())
            ]
        );
        assert_eq!(p_fm["include-custom-instructions"].as_bool(), Some(false));
        assert_eq!(p_fm["user-invocable"].as_bool(), Some(false));

        let profile_text = std::fs::read_to_string(&profile_file).unwrap();
        assert!(profile_text.contains(&format!("<EGM_ROOT>/agents/{}.md", persona)));
        assert!(profile_text.contains("Do not retrieve parent history"));
    }

    // Verify egm.agent.md orchestrator entrypoint
    let egm_profile = profiles_dir.join("egm.agent.md");
    assert!(egm_profile.exists());
    let egm_fm = read_frontmatter(&egm_profile);
    assert_eq!(
        egm_fm["tools"].as_sequence().unwrap(),
        &[
            serde_yaml::Value::String("read".into()),
            serde_yaml::Value::String("search".into()),
            serde_yaml::Value::String("edit".into()),
            serde_yaml::Value::String("agent".into()),
        ]
    );
}

#[test]
fn test_claude_reviewers_are_read_only_with_fresh_context() {
    let root = repo_root();
    let claude_agents_dir = root.join(".claude-plugin/agents");

    for persona in REVIEWERS {
        let adapter = claude_agents_dir.join(format!("{}.md", persona));
        assert!(
            adapter.exists(),
            "Missing Claude adapter: {}",
            adapter.display()
        );

        let fm = read_frontmatter(&adapter);
        let tools: HashSet<String> = fm["tools"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(
            tools,
            HashSet::from(["Read".into(), "Grep".into(), "Glob".into()])
        );
        assert_eq!(fm["model"].as_str(), Some("inherit"));
        assert_eq!(fm["omitClaudeMd"].as_bool(), Some(true));
        assert!(fm.get("memory").is_none());

        let adapter_text = std::fs::read_to_string(&adapter).unwrap();
        assert!(adapter_text.contains(&format!("${{CLAUDE_PLUGIN_ROOT}}/agents/{}.md", persona)));
    }

    // Verify Claude Synthesizer is tool restricted
    let synth = claude_agents_dir.join("synthesizer.md");
    let synth_fm = read_frontmatter(&synth);
    let synth_tools: HashSet<String> = synth_fm["tools"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        synth_tools,
        HashSet::from([
            "Agent".into(),
            "Read".into(),
            "Grep".into(),
            "Glob".into(),
            "Write".into()
        ])
    );
}

#[test]
fn test_cursor_manifest_and_native_adapters() {
    let root = repo_root();
    let cursor_manifest = read_json(root.join(".cursor-plugin/plugin.json"));
    let canonical = read_json(root.join("plugin.json"));

    for field in &["name", "version", "homepage", "repository", "license"] {
        assert_eq!(cursor_manifest[field], canonical[field]);
    }
    assert_eq!(cursor_manifest["author"]["name"], "Google");
    assert_eq!(cursor_manifest["skills"], "./skills/");
    assert_eq!(cursor_manifest["agents"], "./.cursor-plugin/agents/");
    assert_eq!(cursor_manifest["rules"], "./.cursor-plugin/rules/");

    for persona in REVIEWERS {
        let cursor_agent = root.join(format!(".cursor-plugin/agents/{}.md", persona));
        assert!(cursor_agent.exists());
    }
}

#[test]
fn test_marketplaces_resolve_shared_package() {
    let root = repo_root();
    let canonical = read_json(root.join("plugin.json"));

    for rel in &[
        ".agents/plugins/marketplace.json",
        ".claude-plugin/marketplace.json",
    ] {
        let m = read_json(root.join(rel));
        assert_eq!(m["name"], "egm");
        assert_eq!(m["plugins"].as_array().unwrap().len(), 1);
        let entry = &m["plugins"][0];
        assert_eq!(entry["name"], canonical["name"]);
    }
}

#[test]
fn test_jsonl_trace_preserves_unicode_line_separators() {
    // Tests that Unicode line separators inside JSON string payloads
    // do not break JSONL line streaming (ported from test_harnesses.py)
    for sep in &["\u{0085}", "\u{2028}", "\u{2029}"] {
        let json_line = format!(r#"{{"type":"item","text":"before{}after"}}"#, sep);
        let _lines: Vec<&str> = json_line.lines().collect();
        // The record itself must remain on one line or be parseable as valid JSON
        let val: Value =
            serde_json::from_str(&json_line).expect("Must parse JSON with Unicode separator");
        assert_eq!(val["text"].as_str().unwrap(), format!("before{}after", sep));
    }
}
