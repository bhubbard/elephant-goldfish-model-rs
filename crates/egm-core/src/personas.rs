// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersonaError {
    #[error("Failed to read persona file {0}: {1}")]
    IoError(String, #[source] std::io::Error),
    #[error("Persona validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersonaRole {
    BootstrapOrchestrator,
    CodebaseDetails,
    Comprehension,
    DesignCritic,
    ImplementationReadiness,
    Structural,
    Style,
    Synthesizer,
    ZoomOut,
}

impl std::str::FromStr for PersonaRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bootstrap_orchestrator" => Ok(Self::BootstrapOrchestrator),
            "codebase_details" => Ok(Self::CodebaseDetails),
            "comprehension" => Ok(Self::Comprehension),
            "design_critic" => Ok(Self::DesignCritic),
            "implementation_readiness" => Ok(Self::ImplementationReadiness),
            "structural" => Ok(Self::Structural),
            "style" => Ok(Self::Style),
            "synthesizer" => Ok(Self::Synthesizer),
            "zoom_out" => Ok(Self::ZoomOut),
            _ => Err(()),
        }
    }
}

impl PersonaRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BootstrapOrchestrator => "bootstrap_orchestrator",
            Self::CodebaseDetails => "codebase_details",
            Self::Comprehension => "comprehension",
            Self::DesignCritic => "design_critic",
            Self::ImplementationReadiness => "implementation_readiness",
            Self::Structural => "structural",
            Self::Style => "style",
            Self::Synthesizer => "synthesizer",
            Self::ZoomOut => "zoom_out",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BootstrapOrchestrator => "Bootstrap Orchestrator",
            Self::CodebaseDetails => "Codebase Details Auditor",
            Self::Comprehension => "Comprehension Reviewer",
            Self::DesignCritic => "Design Critic",
            Self::ImplementationReadiness => "Implementation Readiness Reviewer",
            Self::Structural => "Structural Validator",
            Self::Style => "Style Editor",
            Self::Synthesizer => "Synthesizer",
            Self::ZoomOut => "Zoom-Out Strategist",
        }
    }

    pub fn all() -> &'static [PersonaRole] {
        &[
            Self::BootstrapOrchestrator,
            Self::CodebaseDetails,
            Self::Comprehension,
            Self::DesignCritic,
            Self::ImplementationReadiness,
            Self::Structural,
            Self::Style,
            Self::Synthesizer,
            Self::ZoomOut,
        ]
    }

    pub fn reviewers() -> &'static [PersonaRole] {
        &[
            Self::Comprehension,
            Self::DesignCritic,
            Self::ZoomOut,
            Self::CodebaseDetails,
            Self::ImplementationReadiness,
            Self::Structural,
            Self::Style,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Persona {
    pub role: PersonaRole,
    pub path: PathBuf,
    pub prompt: String,
}

impl Persona {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, PersonaError> {
        let p = path.as_ref();
        let file_stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let role: PersonaRole = file_stem.parse().map_err(|_| {
            PersonaError::ValidationError(format!(
                "Unknown persona role from filename: {}",
                p.display()
            ))
        })?;

        let prompt = std::fs::read_to_string(p)
            .map_err(|e| PersonaError::IoError(p.display().to_string(), e))?;

        if role == PersonaRole::Style {
            // Validate required editorial calibration actions
            for action in &["DROP", "CALIBRATE", "RELOCATE", "KEEP"] {
                let pattern = format!("`{}`", action);
                if !prompt.contains(&pattern) {
                    return Err(PersonaError::ValidationError(format!(
                        "Missing editorial calibration action `{}` in {}",
                        action,
                        p.display()
                    )));
                }
            }
            if !prompt.contains("Defensive Caveats & Uncalibrated Claims") {
                return Err(PersonaError::ValidationError(format!(
                    "Missing 'Defensive Caveats & Uncalibrated Claims' section in {}",
                    p.display()
                )));
            }
        }

        Ok(Persona {
            role,
            path: p.to_path_buf(),
            prompt,
        })
    }
}

pub fn load_all_personas<P: AsRef<Path>>(agents_dir: P) -> Result<Vec<Persona>, PersonaError> {
    let p = agents_dir.as_ref();
    let mut personas = Vec::new();
    for role in PersonaRole::all() {
        let file = p.join(format!("{}.md", role.as_str()));
        if file.exists() {
            personas.push(Persona::load_from_file(&file)?);
        } else {
            return Err(PersonaError::ValidationError(format!(
                "Required persona file missing: {}",
                file.display()
            )));
        }
    }
    Ok(personas)
}
