// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviewError {
    #[error("I/O error reading target spec {0}: {1}")]
    Io(String, #[source] std::io::Error),
    #[error("Review execution error: {0}")]
    Execution(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FindingSeverity {
    Critical,
    Major,
    Minor,
    Suggestion,
}

impl FindingSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::Major => "MAJOR",
            Self::Minor => "MINOR",
            Self::Suggestion => "SUGGESTION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingCategory {
    Comprehension,
    DesignCritique,
    ImplementationReadiness,
    CodebaseAlignment,
    Structural,
    Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRecommendation {
    Drop,
    Calibrate,
    Relocate,
    Keep,
}

impl ActionRecommendation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Drop => "DROP",
            Self::Calibrate => "CALIBRATE",
            Self::Relocate => "RELOCATE",
            Self::Keep => "KEEP",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub stage: u8,
    pub reviewer: String,
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub location: Option<String>,
    pub action: Option<ActionRecommendation>,
    pub description: String,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub target_document: String,
    pub timestamp: String,
    pub stages_evaluated: Vec<u8>,
    pub findings: Vec<Finding>,
    pub summary: String,
    pub approved: bool,
}

impl ReviewReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# EGM Goldfish Review Report\n\n");
        md.push_str(&format!(
            "* **Target Specification:** `{}`\n",
            self.target_document
        ));
        md.push_str(&format!("* **Timestamp:** {}\n", self.timestamp));
        md.push_str(&format!(
            "* **Status:** {}\n\n",
            if self.approved {
                "APPROVED (Ready for Implementation)"
            } else {
                "REVISION REQUIRED"
            }
        ));

        let critical_count = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .count();
        let major_count = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Major)
            .count();
        let minor_count = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Minor)
            .count();
        let suggestion_count = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Suggestion)
            .count();

        md.push_str("## Executive Summary\n\n");
        md.push_str(&format!("{}\n\n", self.summary));

        md.push_str("| Severity | Count |\n| :--- | :--- |\n");
        md.push_str(&format!("| Critical | {} |\n", critical_count));
        md.push_str(&format!("| Major | {} |\n", major_count));
        md.push_str(&format!("| Minor | {} |\n", minor_count));
        md.push_str(&format!("| Suggestion | {} |\n\n", suggestion_count));

        if self.findings.is_empty() {
            md.push_str("> [!TIP]\n> No issues identified. Design satisfies all goldfish reviewer rubrics.\n");
            return md;
        }

        md.push_str("## Detailed Reviewer Findings\n\n");
        for (i, finding) in self.findings.iter().enumerate() {
            let action_str = match &finding.action {
                Some(a) => format!(" `[{}]`", a.as_str()),
                None => String::new(),
            };
            let loc_str = match &finding.location {
                Some(l) => format!(" @ {}", l),
                None => String::new(),
            };

            md.push_str(&format!(
                "### {}. [{}] {}{}{}\n\n",
                i + 1,
                finding.severity.as_str(),
                finding.reviewer,
                action_str,
                loc_str
            ));
            md.push_str(&format!("{}\n\n", finding.description));
            if let Some(fix) = &finding.suggested_fix {
                md.push_str(&format!("**Suggested Resolution:**\n{}\n\n", fix));
            }
        }

        md
    }
}

pub struct ReviewEngine;

impl ReviewEngine {
    pub fn review_spec_file<P: AsRef<Path>>(spec_path: P) -> Result<ReviewReport, ReviewError> {
        let p = spec_path.as_ref();
        let content =
            std::fs::read_to_string(p).map_err(|e| ReviewError::Io(p.display().to_string(), e))?;

        Self::review_content(p.display().to_string(), &content)
    }

    pub fn review_content(
        document_name: String,
        content: &str,
    ) -> Result<ReviewReport, ReviewError> {
        let mut findings = Vec::new();
        let mut id_counter = 1;

        // Stage 1: Comprehension & Heilmeier checks
        let lower = content.to_lowercase();
        let has_problem =
            lower.contains("problem") || lower.contains("overview") || lower.contains("goal");
        let has_tradeoffs = lower.contains("trade-off")
            || lower.contains("tradeoff")
            || lower.contains("alternatives");
        let has_costs =
            lower.contains("cost") || lower.contains("effort") || lower.contains("budget");
        let has_risks =
            lower.contains("risk") || lower.contains("failure") || lower.contains("drawback");

        if !has_problem {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 1,
                reviewer: "Comprehension Reviewer".to_string(),
                severity: FindingSeverity::Critical,
                category: FindingCategory::Comprehension,
                location: Some("Document Root".to_string()),
                action: None,
                description: "Specification lacks an explicit problem statement or goal definition.".to_string(),
                suggested_fix: Some("Add a 'Problem Statement' section answering Heilmeier question #1: What are you trying to do?".to_string()),
            });
            id_counter += 1;
        }

        if !has_tradeoffs {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 1,
                reviewer: "Comprehension Reviewer".to_string(),
                severity: FindingSeverity::Major,
                category: FindingCategory::Comprehension,
                location: None,
                action: None,
                description: "No architectural trade-offs or considered alternatives are documented.".to_string(),
                suggested_fix: Some("Add an 'Alternatives Considered' section detailing rejected options and reasons.".to_string()),
            });
            id_counter += 1;
        }

        if !has_risks {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 2,
                reviewer: "Design Critic".to_string(),
                severity: FindingSeverity::Major,
                category: FindingCategory::DesignCritique,
                location: None,
                action: None,
                description: "Risks and failure modes are omitted from the design.".to_string(),
                suggested_fix: Some("Document worst-case operational scenarios, failure boundaries, and disaster recovery.".to_string()),
            });
            id_counter += 1;
        }

        if !has_costs {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 2,
                reviewer: "Zoom-Out Strategist".to_string(),
                severity: FindingSeverity::Minor,
                category: FindingCategory::DesignCritique,
                location: None,
                action: None,
                description:
                    "Cost, effort, or resource estimation is omitted from the specification."
                        .to_string(),
                suggested_fix: Some(
                    "Add an estimate of engineering timeline and resource costs (Heilmeier #6)."
                        .to_string(),
                ),
            });
            id_counter += 1;
        }

        // Stage 3: Implementation Readiness & Verification
        let has_tests = lower.contains("test")
            || lower.contains("verification")
            || lower.contains("validation");
        let has_migration = lower.contains("migration")
            || lower.contains("rollback")
            || lower.contains("backwards");

        if !has_tests {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 3,
                reviewer: "Implementation Readiness Reviewer".to_string(),
                severity: FindingSeverity::Critical,
                category: FindingCategory::ImplementationReadiness,
                location: None,
                action: None,
                description: "Specification does not describe testing or automated verification strategy.".to_string(),
                suggested_fix: Some("Add a 'Verification Plan' section detailing unit, integration, and contract tests.".to_string()),
            });
            id_counter += 1;
        }

        if !has_migration {
            findings.push(Finding {
                id: format!("FINDING-{:03}", id_counter),
                stage: 3,
                reviewer: "Implementation Readiness Reviewer".to_string(),
                severity: FindingSeverity::Minor,
                category: FindingCategory::ImplementationReadiness,
                location: None,
                action: None,
                description: "No explicit rollback or migration plan is provided.".to_string(),
                suggested_fix: Some("Clarify whether database/schema migrations or breaking interface changes require rollbacks.".to_string()),
            });
            id_counter += 1;
        }

        // Stage 4: Style Editor - audit defensive caveats
        let defensive_words = [
            "might possibly",
            "could potentially",
            "we hope that",
            "it seems that",
        ];
        for word in &defensive_words {
            if lower.contains(word) {
                findings.push(Finding {
                    id: format!("FINDING-{:03}", id_counter),
                    stage: 4,
                    reviewer: "Style Editor".to_string(),
                    severity: FindingSeverity::Minor,
                    category: FindingCategory::Style,
                    location: None,
                    action: Some(ActionRecommendation::Calibrate),
                    description: format!("Uncalibrated defensive claim detected: '{}'.", word),
                    suggested_fix: Some("State technical invariants or empirical probabilities directly rather than hedged prose.".to_string()),
                });
                id_counter += 1;
            }
        }

        let approved = findings
            .iter()
            .all(|f| f.severity != FindingSeverity::Critical);

        let summary = if approved && findings.is_empty() {
            "Specification fully approved across all Goldfish reviewer stages with zero findings."
                .to_string()
        } else if approved {
            format!(
                "Specification provisionally approved with {} non-critical finding(s). Implementation may proceed alongside revisions.",
                findings.len()
            )
        } else {
            format!(
                "Specification rejected due to {} critical finding(s). Revisions required before implementation.",
                findings.iter().filter(|f| f.severity == FindingSeverity::Critical).count()
            )
        };

        Ok(ReviewReport {
            target_document: document_name,
            timestamp: Utc::now().to_rfc3339(),
            stages_evaluated: vec![1, 2, 3, 4, 5],
            findings,
            summary,
            approved,
        })
    }
}
