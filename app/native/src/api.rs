use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use loose_ends_core::{
    models::{Direction, ExtractDirection, FieldConfidence, Provenance, RawInputType},
    planner::PlannerConfig,
    store::NewCommitment,
    Store,
};

#[cfg(feature = "frb-neural")]
use loose_ends_core::neural::{NeuralExtractor, ProvenancePath};

pub struct CoreApi {
    store: Arc<Mutex<Store>>,
    #[cfg(feature = "frb-neural")]
    extractor: Arc<NeuralExtractor>,
}

impl CoreApi {
    pub fn open(path: String) -> Result<Self, String> {
        let store = Store::open(&path).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            #[cfg(feature = "frb-neural")]
            extractor: Arc::new(NeuralExtractor::default()),
        })
    }

    pub async fn ingest_text(
        &self,
        text: String,
        today: String,
        model_path: Option<String>,
    ) -> Result<String, String> {
        let date = parse_date(&today)?;
        let store = Arc::clone(&self.store);

        #[cfg(feature = "frb-neural")]
        {
            if let Some(model_path) = model_path.filter(|p| !p.trim().is_empty()) {
                let extractor = Arc::clone(&self.extractor);
                let path = model_path.clone();
                return tokio::task::spawn_blocking(move || {
                    ingest_with_neural(&store, &extractor, &text, date, &path)
                })
                .await
                .map_err(|e| e.to_string())?;
            }
        }

        tokio::task::spawn_blocking(move || ingest_rules_only(&store, &text, date))
            .await
            .map_err(|e| e.to_string())?
    }

    pub async fn list_drafts(&self) -> Result<String, String> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            let drafts = store.list_drafts().map_err(|e| e.to_string())?;
            serde_json::to_string(&drafts).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn confirm_draft(
        &self,
        id: i64,
        description: Option<String>,
        direction: Option<String>,
        expected_date: Option<String>,
        party: Option<String>,
    ) -> Result<Option<i64>, String> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let direction = direction
                .as_deref()
                .map(parse_direction)
                .transpose()?;
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            store
                .confirm_draft(
                    id,
                    description.as_deref(),
                    direction,
                    expected_date.as_deref(),
                    party.as_deref(),
                )
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn list_open(
        &self,
        direction: String,
        today: String,
    ) -> Result<String, String> {
        let direction = parse_direction(&direction)?;
        let today = parse_date(&today)?;
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            let values = store
                .view(direction, today, &PlannerConfig::default())
                .map_err(|e| e.to_string())?;
            let output = values
                .into_iter()
                .map(|(commitment, action)| {
                    serde_json::json!({
                        "id": commitment.id,
                        "description": commitment.description,
                        "direction": commitment.direction.as_str(),
                        "expected_date": commitment.expected_date,
                        "party": match direction {
                            Direction::UserOwes => commitment.owed_to,
                            Direction::OwedToUser => commitment.owed_by,
                        },
                        "aging_action": format!("{action:?}"),
                        "created_at": commitment.created_at,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::to_string(&output).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn create_commitment(
        &self,
        description: String,
        direction: String,
        expected_date: Option<String>,
        party: Option<String>,
    ) -> Result<i64, String> {
        let direction = parse_direction(&direction)?;
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            let new = match direction {
                Direction::UserOwes => NewCommitment {
                    description: &description,
                    direction,
                    expected_date: expected_date.as_deref(),
                    owed_by_party: Some("user"),
                    owed_to_party: party.as_deref(),
                    provenance: Provenance::Manual,
                    confidence: Default::default(),
                },
                Direction::OwedToUser => NewCommitment {
                    description: &description,
                    direction,
                    expected_date: expected_date.as_deref(),
                    owed_by_party: party.as_deref(),
                    owed_to_party: Some("user"),
                    provenance: Provenance::Manual,
                    confidence: Default::default(),
                },
            };
            store.create_commitment(new).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn resolve_commitment(
        &self,
        id: i64,
        note: Option<String>,
    ) -> Result<bool, String> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            store
                .resolve_commitment(id, note.as_deref())
                .map(|_| true)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub async fn snooze_commitment(&self, id: i64) -> Result<bool, String> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
            store
                .snooze_commitment(id)
                .map(|_| true)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| e.to_string())
}

fn parse_direction(value: &str) -> Result<Direction, String> {
    match value {
        "user_owes" => Ok(Direction::UserOwes),
        "owed_to_user" => Ok(Direction::OwedToUser),
        _ => Err("direction must be user_owes or owed_to_user".to_string()),
    }
}

fn ingest_rules_only(
    store: &Arc<Mutex<Store>>,
    text: &str,
    today: NaiveDate,
) -> Result<String, String> {
    let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
    let report = store.ingest_text_rules(text, today).map_err(|e| e.to_string())?;
    let drafts = store.list_drafts().map_err(|e| e.to_string())?;
    let selected = drafts
        .into_iter()
        .filter(|d| report.draft_ids.contains(&d.id))
        .collect::<Vec<_>>();
    serde_json::to_string(&selected).map_err(|e| e.to_string())
}

#[cfg(feature = "frb-neural")]
fn ingest_with_neural(
    store: &Arc<Mutex<Store>>,
    extractor: &Arc<NeuralExtractor>,
    text: &str,
    today: NaiveDate,
    model_path: &str,
) -> Result<String, String> {
    let (candidates, path) = extractor.extract_with_model_path(text, today, model_path);
    let provenance = match path {
        ProvenancePath::Model => Provenance::ModelExtracted,
        ProvenancePath::RuleFallbackTimeout
        | ProvenancePath::RuleFallbackFailure
        | ProvenancePath::RuleFallbackBreakerOpen => Provenance::RuleExtracted,
    };
    let store = store.lock().map_err(|_| "store lock poisoned".to_string())?;
    let source_id = store
        .add_entry_source(RawInputType::Text)
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for candidate in candidates {
        let id = store
            .add_draft(
                &candidate.description,
                candidate.direction_symbolic,
                candidate.expected_date.as_deref(),
                candidate.party.as_deref(),
                provenance,
                &candidate.confidence,
                Some(source_id),
            )
            .map_err(|e| e.to_string())?;
        out.push(serde_json::json!({
            "id": id,
            "description": candidate.description,
            "direction": match candidate.direction_symbolic {
                ExtractDirection::UserOwes => "user_owes",
                ExtractDirection::OwedToUser => "owed_to_user",
                ExtractDirection::Unclear => "unclear",
            },
            "expected_date": candidate.expected_date,
            "party": candidate.party,
            "party_confidence": match candidate.confidence.party {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "date_confidence": match candidate.confidence.date {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "overall_confidence": match candidate.confidence.overall {
                Some(FieldConfidence::High) => "high",
                _ => "low",
            },
            "source_provenance": provenance.as_str(),
        }));
    }
    serde_json::to_string(&out).map_err(|e| e.to_string())
}
