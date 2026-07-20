//! Core data models for token risk assessment.
//!
//! Every field in every input is treated as strictly-typed structured data,
//! never as natural language that could be "reinterpreted". This is the
//! structural defence against prompt injection.

use serde::{Deserialize, Serialize};

/// Risk assessment traffic-light status.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum RagStatus {
    Green,
    Amber,
    Red,
}

/// An individual risk finding.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Finding {
    /// Machine-readable finding code (e.g. "FREEZE_AUTHORITY_ACTIVE").
    pub code: String,
    /// Severity of this individual finding.
    pub severity: RagStatus,
    /// Human-readable explanation, ≤ 1 sentence.
    pub detail: String,
}

/// Complete risk assessment report for a token mint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskReport {
    /// The mint address that was checked.
    pub mint: String,
    /// Overall risk status (worst severity across all findings).
    pub status: RagStatus,
    /// Individual findings.
    pub findings: Vec<Finding>,
    /// LLM-friendly summary, hard-capped at ~200 tokens (~800 chars).
    pub summary: String,
}

/// Aggregate findings into the worst status: RED > AMBER > GREEN.
pub fn aggregate_status(findings: &[Finding]) -> RagStatus {
    if findings.iter().any(|f| f.severity == RagStatus::Red) {
        RagStatus::Red
    } else if findings.iter().any(|f| f.severity == RagStatus::Amber) {
        RagStatus::Amber
    } else {
        RagStatus::Green
    }
}

/// Render a compact summary for LLM consumption.
/// Hard-capped at 800 characters to avoid flooding context window.
pub fn render_summary(mint: &str, status: RagStatus, findings: &[Finding]) -> String {
    let status_str = match status {
        RagStatus::Green => "GREEN",
        RagStatus::Amber => "AMBER",
        RagStatus::Red => "RED",
    };

    let mut summary = format!("{status_str}: Token {mint}.");

    if findings.is_empty() {
        summary.push_str(" No risk indicators found.");
    } else {
        for f in findings {
            let line = format!(" [{}] {}", f.code, f.detail);
            if summary.len() + line.len() > 780 {
                summary.push_str(" ... (truncated)");
                break;
            }
            summary.push_str(&line);
        }
    }

    // Hard cap at 800 characters
    if summary.len() > 800 {
        summary.truncate(797);
        summary.push_str("...");
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_empty_is_green() {
        assert_eq!(aggregate_status(&[]), RagStatus::Green);
    }

    #[test]
    fn aggregate_single_amber() {
        let findings = vec![Finding {
            code: "TEST".into(),
            severity: RagStatus::Amber,
            detail: "test".into(),
        }];
        assert_eq!(aggregate_status(&findings), RagStatus::Amber);
    }

    #[test]
    fn aggregate_red_overrides_amber() {
        let findings = vec![
            Finding { code: "A".into(), severity: RagStatus::Amber, detail: "a".into() },
            Finding { code: "B".into(), severity: RagStatus::Red, detail: "b".into() },
            Finding { code: "C".into(), severity: RagStatus::Green, detail: "c".into() },
        ];
        assert_eq!(aggregate_status(&findings), RagStatus::Red);
    }

    #[test]
    fn summary_is_capped() {
        let mut findings = Vec::new();
        for i in 0..100 {
            findings.push(Finding {
                code: format!("FINDING_{i}"),
                severity: RagStatus::Amber,
                detail: format!("This is a very long finding detail number {i} with lots of text."),
            });
        }
        let summary = render_summary("TestMint", RagStatus::Amber, &findings);
        assert!(summary.len() <= 800);
    }
}
