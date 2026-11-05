// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StandardError {
    #[error("Failed to read standard file {0}: {1}")]
    IoError(String, #[source] std::io::Error),
    #[error("Standard validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardType {
    FeatureSpec,
    Heilmeier,
    ImplementationReadiness,
    ReImplementation,
    ZoomOut,
}

impl StandardType {
    pub fn as_filename(&self) -> &'static str {
        match self {
            Self::FeatureSpec => "feature-spec-standard.md",
            Self::Heilmeier => "heilmeier-standard.md",
            Self::ImplementationReadiness => "implementation-readiness-standard.md",
            Self::ReImplementation => "re-implementation-standard.md",
            Self::ZoomOut => "zoom-out-standard.md",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FeatureSpec => "Feature Specification Standard",
            Self::Heilmeier => "Heilmeier Catechism Standard",
            Self::ImplementationReadiness => "Implementation Readiness Standard",
            Self::ReImplementation => "Re-Implementation Standard",
            Self::ZoomOut => "Zoom-Out Architectural Standard",
        }
    }

    pub fn all() -> &'static [StandardType] {
        &[
            Self::FeatureSpec,
            Self::Heilmeier,
            Self::ImplementationReadiness,
            Self::ReImplementation,
            Self::ZoomOut,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct DocumentStandard {
    pub standard_type: StandardType,
    pub path: PathBuf,
    pub content: String,
}

impl DocumentStandard {
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<Self>, StandardError> {
        let dir_path = dir.as_ref();
        let mut standards = Vec::new();
        for std_type in StandardType::all() {
            let file = dir_path.join(std_type.as_filename());
            if !file.exists() {
                return Err(StandardError::ValidationError(format!(
                    "Missing document standard: {}",
                    file.display()
                )));
            }
            let content = std::fs::read_to_string(&file)
                .map_err(|e| StandardError::IoError(file.display().to_string(), e))?;
            standards.push(DocumentStandard {
                standard_type: *std_type,
                path: file,
                content,
            });
        }
        Ok(standards)
    }
}
