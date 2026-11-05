// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use crate::standards::StandardType;
use chrono::Utc;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DraftError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File already exists: {0}")]
    AlreadyExists(String),
}

pub struct DraftGenerator;

impl DraftGenerator {
    pub fn scaffold_spec(title: &str, std_type: StandardType, author: &str) -> String {
        let date = Utc::now().format("%Y-%m-%d").to_string();

        match std_type {
            StandardType::Heilmeier => format!(
                r#"# {title}

* **Author:** {author}
* **Status:** Draft
* **Standard:** Heilmeier Catechism
* **Date:** {date}

---

## 1. What are you trying to do?
Articulate your objectives using absolutely no jargon.

## 2. How is it done today, and what are the limits of current practice?
Current state of the art, limitations, and baseline constraints.

## 3. What is new in your approach, and why do you think it will be successful?
Key technical novelty, architectural innovations, and core hypotheses.

## 4. Who cares? If you succeed, what difference will it make?
Target users, operational impact, and business value.

## 5. What are the risks?
Failure modes, unstated assumptions, dependencies, and external failure domains.

## 6. How much will it cost? How long will it take?
Resource allocation, engineering complexity, and delivery timeline.

## 7. What are the mid-term and final exams to check for success?
Measurable verification gates, end-to-end testing, and rollback criteria.
"#
            ),
            StandardType::FeatureSpec => format!(
                r#"# Feature Spec: {title}

* **Author:** {author}
* **Status:** Draft
* **Standard:** Feature Specification Standard
* **Date:** {date}

---

## Context & Objectives
Background context, user journeys, and non-goals.

## Architecture & System Design
Component model, state lifecycle, sequence diagrams, and interface definitions.

## Data Model & Interfaces
Types, schema definitions, API routes, and serialization formats.

## Failure Domains & Mitigations
Error propagation, timeouts, idempotency, and fallback behavior.

## Verification & Testing Plan
Unit coverage, integration scenarios, and automated CI assertions.
"#
            ),
            StandardType::ZoomOut => format!(
                r#"# System Architecture: {title}

* **Author:** {author}
* **Status:** Draft
* **Standard:** Zoom-Out Architectural Standard
* **Date:** {date}

---

## High-Level Topology
Macro view of services, boundaries, and network topology.

## Component Responsibilities
Strict responsibility boundaries and data flows.

## Interface Contracts & Protocols
Synchronous vs asynchronous communication channels and protocol specifications.

## Evolution & Deprecation
Upgrade paths, schema versioning, and migration milestones.
"#
            ),
            _ => format!(
                r#"# Technical Design: {title}

* **Author:** {author}
* **Status:** Draft
* **Date:** {date}

---

## Overview & Goals
## Architecture & Design
## Verification Plan
"#
            ),
        }
    }

    pub fn scaffold_brainstorm_brief(idea: &str, author: &str) -> String {
        let date = Utc::now().format("%Y-%m-%d").to_string();

        format!(
            r#"# Concept Brief: {idea}

* **Author:** {author}
* **Status:** Brainstorming
* **Date:** {date}

---

## Problem Statement
What specific challenge or bottleneck are we addressing?

## 6-Lens Shootdown Analysis

### 1. Inversion Lens
What guarantees failure if we proceed? How do we design against it?

### 2. First Principles Lens
What are the fundamental physical and computational limits of this problem?

### 3. Worst-Case Lens
What happens if dependency systems experience a hard outage or data corruption?

### 4. Simplicity Lens (YAGNI)
What is the minimal viable solution that solves 80% of the problem with 20% of the code?

### 5. Prior Art Lens
How have similar distributed or systems software solved this previously?

### 6. User-Centric Lens
How does this change the developer ergonomic experience or end-user workflow?

## Agreed Direction & Next Steps
Agreed architectural path forward to draft in `design_draft.md`.
"#
        )
    }

    pub fn write_to_file<P: AsRef<Path>>(
        path: P,
        content: &str,
        force: bool,
    ) -> Result<PathBuf, DraftError> {
        let p = path.as_ref();
        if p.exists() && !force {
            return Err(DraftError::AlreadyExists(p.display().to_string()));
        }
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, content)?;
        Ok(p.to_path_buf())
    }
}
