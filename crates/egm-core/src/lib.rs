// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

//! # egm-core
//!
//! Native Rust implementation of the Elephant-Goldfish Model (EGM) Spec-Driven Development framework.
//!
//! This crate provides:
//! - Specification and Persona models for EGM.
//! - Plugin manifest loading and cross-target synchronization validation.
//! - Offline repository and skill validator (frontmatter, 24KB limits, table syntax, link integrity).
//! - Deterministic package and zip builder for Claude and Codex/OpenAI runtimes.
//! - Release management and SHA-256 digest creation.
//! - Multi-stage Goldfish reviewer pipeline and findings synthesis.
//! - Codebase drift analysis against living specifications.
//! - Project ecosystem inspection and documentation bootstrapping.

pub mod bootstrap;
pub mod draft;
pub mod drift;
pub mod manifest;
pub mod packaging;
pub mod personas;
pub mod review;
pub mod skills;
pub mod standards;
pub mod validator;

pub use bootstrap::{Bootstrapper, ProjectEcosystem, ProjectInspection};
pub use draft::DraftGenerator;
pub use drift::{DriftAnalyzer, DriftItem, DriftReport};
pub use manifest::{ClaudePluginManifest, CodexPluginManifest, PluginManifest};
pub use packaging::{build_package, prepare_release, validate_version_for_tag};
pub use personas::{load_all_personas, Persona, PersonaRole};
pub use review::{
    ActionRecommendation, Finding, FindingCategory, FindingSeverity, ReviewEngine, ReviewReport,
};
pub use skills::{
    discover_skills, extract_frontmatter_and_body, Skill, SkillFrontmatter, MAX_SKILL_BODY_BYTES,
};
pub use standards::{DocumentStandard, StandardType};
pub use validator::{CheckResult, CheckStatus, ValidationReport, Validator};
