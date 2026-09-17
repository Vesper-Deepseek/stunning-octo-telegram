//! Core domain types for Loose Ends (spec Section 5).

use serde::{Deserialize, Serialize};

pub const DIRECTION_USER_OWES: &str = "user_owes";
pub const DIRECTION_OWED_TO_USER: &str = "owed_to_user";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    UserOwes,
    OwedToUser,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::UserOwes => DIRECTION_USER_OWES,
            Direction::OwedToUser => DIRECTION_OWED_TO_USER,
        }
    }
}

/// Neural extractor may report "unclear"; only the two firm directions can
/// ever be persisted into the commitment table (spec Section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractDirection {
    UserOwes,
    OwedToUser,
    Unclear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Open,
    Overdue,
    Resolved,
    Snoozed,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Overdue => "overdue",
            Status::Resolved => "resolved",
            Status::Snoozed => "snoozed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    ModelExtracted,
    RuleExtracted,
    Manual,
}

impl Provenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provenance::ModelExtracted => "model_extracted",
            Provenance::RuleExtracted => "rule_extracted",
            Provenance::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawInputType {
    Text,
    Screenshot,
    Voice,
    Manual,
}

impl RawInputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RawInputType::Text => "text",
            RawInputType::Screenshot => "screenshot",
            RawInputType::Voice => "voice",
            RawInputType::Manual => "manual",
        }
    }
}

/// Per-field confidence scores (stored as confidence_json on commitments).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Confidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<FieldConfidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<FieldConfidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall: Option<FieldConfidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldConfidence {
    High,
    Low,
}

/// A saved fact: a confirmed bidirectional obligation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub id: i64,
    pub description: String,
    pub direction: Direction,
    /// ISO 8601 date or None. Chrono NaiveDate rendered as YYYY-MM-DD.
    pub expected_date: Option<String>,
    pub status: Status,
    pub created_at: String,
    pub last_updated_at: String,
    pub source_provenance: Provenance,
    pub confidence_json: String,
    /// joined party labels for convenience views
    pub owed_by: Option<String>,
    pub owed_to: Option<String>,
}

/// A pending extraction awaiting user review. Never treated as fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftCommitment {
    pub id: i64,
    pub description: String,
    pub direction: ExtractDirection,
    pub expected_date: Option<String>,
    pub party_guess: Option<String>,
    pub created_at: String,
    pub source_provenance: Provenance,
    pub confidence_json: String,
    pub entry_source_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: i64,
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySource {
    pub id: i64,
    pub raw_input_type: RawInputType,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub id: i64,
    pub resolved_at: String,
    pub resolution_note: Option<String>,
}
