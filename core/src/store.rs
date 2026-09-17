//! Storage layer: SQLite schema per spec Section 5, plus the draft queue
//! that keeps unconfirmed extractions out of the fact table.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::*;

pub struct Store {
    pub(crate) conn: Connection,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS commitment (
    id                INTEGER PRIMARY KEY,
    description       TEXT NOT NULL,
    direction         TEXT NOT NULL CHECK(direction IN ('user_owes','owed_to_user')),
    expected_date     TEXT,
    status            TEXT NOT NULL DEFAULT 'open'
                      CHECK(status IN ('open','overdue','resolved','snoozed')),
    created_at        TEXT NOT NULL,
    last_updated_at   TEXT NOT NULL,
    source_provenance TEXT NOT NULL
                      CHECK(source_provenance IN ('model_extracted','rule_extracted','manual')),
    confidence_json   TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS party (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS entry_source (
    id             INTEGER PRIMARY KEY,
    raw_input_type TEXT NOT NULL CHECK(raw_input_type IN ('text','screenshot','voice','manual')),
    captured_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resolution (
    id              INTEGER PRIMARY KEY,
    resolved_at     TEXT NOT NULL,
    resolution_note TEXT
);

-- relationship types as join records (spec Section 5)
CREATE TABLE IF NOT EXISTS edge_owed_by (
    commitment_id INTEGER NOT NULL REFERENCES commitment(id),
    party_id      INTEGER NOT NULL REFERENCES party(id),
    UNIQUE(commitment_id)
);
CREATE TABLE IF NOT EXISTS edge_owed_to (
    commitment_id INTEGER NOT NULL REFERENCES commitment(id),
    party_id      INTEGER NOT NULL REFERENCES party(id),
    UNIQUE(commitment_id)
);
CREATE TABLE IF NOT EXISTS edge_derived_from (
    commitment_id    INTEGER NOT NULL REFERENCES commitment(id),
    entry_source_id  INTEGER NOT NULL REFERENCES entry_source(id)
);
CREATE TABLE IF NOT EXISTS edge_resolved_by (
    commitment_id INTEGER NOT NULL REFERENCES commitment(id),
    resolution_id INTEGER NOT NULL REFERENCES resolution(id),
    UNIQUE(commitment_id)
);
CREATE TABLE IF NOT EXISTS edge_relates_to (
    from_commitment_id INTEGER NOT NULL REFERENCES commitment(id),
    to_commitment_id   INTEGER NOT NULL REFERENCES commitment(id),
    PRIMARY KEY (from_commitment_id, to_commitment_id)
);

-- review queue: extractions not yet confirmed by the user
CREATE TABLE IF NOT EXISTS draft_commitment (
    id                INTEGER PRIMARY KEY,
    description       TEXT NOT NULL,
    direction         TEXT NOT NULL CHECK(direction IN ('user_owes','owed_to_user','unclear')),
    expected_date     TEXT,
    party_guess       TEXT,
    created_at        TEXT NOT NULL,
    source_provenance TEXT NOT NULL
                      CHECK(source_provenance IN ('model_extracted','rule_extracted','manual')),
    confidence_json   TEXT NOT NULL DEFAULT '{}',
    entry_source_id   INTEGER REFERENCES entry_source(id)
);
"#;

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    // ---------------- parties ----------------

    pub fn upsert_party(&self, name: &str) -> rusqlite::Result<i64> {
        self.conn
            .execute("INSERT OR IGNORE INTO party(name) VALUES (?1)", params![name])?;
        Ok(self
            .conn
            .query_row("SELECT id FROM party WHERE name = ?1", params![name], |r| {
                r.get(0)
            })?)
    }

    // ---------------- entry sources ----------------

    pub fn add_entry_source(&self, t: RawInputType) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO entry_source(raw_input_type, captured_at) VALUES (?1, ?2)",
            params![t.as_str(), now_iso()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ---------------- manual CRUD (spec Stage 2) ----------------

    pub fn create_commitment(&self, nc: NewCommitment) -> rusqlite::Result<i64> {
        let conf = serde_json::to_string(&nc.confidence).unwrap_or_else(|_| "{}".into());
        let ts = now_iso();
        if !matches!(nc.direction, Direction::UserOwes | Direction::OwedToUser) {
            return Err(rusqlite::Error::InvalidParameterName(
                "direction must be user_owes or owed_to_user".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO commitment(description, direction, expected_date, status,
             created_at, last_updated_at, source_provenance, confidence_json)
             VALUES (?1,?2,?3,'open',?4,?4,?5,?6)",
            params![
                nc.description,
                nc.direction.as_str(),
                nc.expected_date,
                ts,
                nc.provenance.as_str(),
                conf
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        if let Some(p) = nc.owed_by_party {
            let pid = self.upsert_party(p)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO edge_owed_by(commitment_id, party_id) VALUES (?1,?2)",
                params![id, pid],
            )?;
        }
        if let Some(p) = nc.owed_to_party {
            let pid = self.upsert_party(p)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO edge_owed_to(commitment_id, party_id) VALUES (?1,?2)",
                params![id, pid],
            )?;
        }
        Ok(id)
    }

    pub fn get_commitment(&self, id: i64) -> rusqlite::Result<Option<Commitment>> {
        self.conn
            .query_row(GET_ONE, params![id], Self::row_to_commitment)
            .optional()
    }

    pub fn update_commitment_fields(
        &self,
        id: i64,
        description: Option<&str>,
        expected_date: Option<Option<&str>>,
        direction: Option<Direction>,
    ) -> rusqlite::Result<()> {
        let cur = self
            .get_commitment(id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let desc = description.map(str::to_string).unwrap_or(cur.description);
        let date = match expected_date {
            Some(d) => d.map(str::to_string),
            None => cur.expected_date,
        };
        let dir = direction.unwrap_or(cur.direction);
        self.conn.execute(
            "UPDATE commitment SET description=?1, expected_date=?2, direction=?3,
             last_updated_at=?4 WHERE id=?5",
            params![desc, date, dir.as_str(), now_iso(), id],
        )?;
        Ok(())
    }

    pub fn resolve_commitment(&self, id: i64, note: Option<&str>) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO resolution(resolved_at, resolution_note) VALUES (?1, ?2)",
            params![now_iso(), note],
        )?;
        let rid = self.conn.last_insert_rowid();
        self.conn.execute(
            "UPDATE commitment SET status='resolved', last_updated_at=?2 WHERE id=?1",
            params![id, now_iso()],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO edge_resolved_by(commitment_id, resolution_id) VALUES (?1,?2)",
            params![id, rid],
        )?;
        Ok(())
    }

    pub fn snooze_commitment(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE commitment SET status='snoozed', last_updated_at=?2 WHERE id=?1",
            params![id, now_iso()],
        )?;
        Ok(())
    }

    pub fn reopen_commitment(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE commitment SET status='open', last_updated_at=?2 WHERE id=?1",
            params![id, now_iso()],
        )?;
        Ok(())
    }

    /// Mark stale open commitments overdue (expected_date < today).
    pub fn refresh_overdue(&self, today: &str) -> rusqlite::Result<usize> {
        self.conn.execute(
            "UPDATE commitment SET status='overdue', last_updated_at=?2
             WHERE status IN ('open') AND expected_date IS NOT NULL AND expected_date < ?1",
            params![today, now_iso()],
        )
    }

    pub fn list_open(&self, dir: Direction) -> rusqlite::Result<Vec<Commitment>> {
        let mut stmt = self.conn.prepare(&format!(
            "{} WHERE c.status IN ('open','overdue') AND c.direction = ?1
             ORDER BY COALESCE(c.expected_date, '9999-12-31') ASC",
            GET_BASE
        ))?;
        let rows = stmt.query_map(params![dir.as_str()], Self::row_to_commitment)?;
        rows.collect()
    }

    pub fn list_all(&self) -> rusqlite::Result<Vec<Commitment>> {
        let mut stmt = self.conn.prepare(&format!("{} ORDER BY c.id", GET_BASE))?;
        let rows = stmt.query_map([], Self::row_to_commitment)?;
        rows.collect()
    }

    pub fn relates(&self, from: i64, to: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO edge_relates_to(from_commitment_id, to_commitment_id) VALUES (?1,?2)",
            params![from, to],
        )?;
        Ok(())
    }

    // ---------------- drafts ----------------

    #[allow(clippy::too_many_arguments)]
    pub fn add_draft(
        &self,
        description: &str,
        direction: ExtractDirection,
        expected_date: Option<&str>,
        party_guess: Option<&str>,
        provenance: Provenance,
        confidence: &Confidence,
        entry_source_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let conf = serde_json::to_string(confidence).unwrap_or_else(|_| "{}".into());
        self.conn.execute(
            "INSERT INTO draft_commitment(description, direction, expected_date, party_guess,
             created_at, source_provenance, confidence_json, entry_source_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                description,
                match direction {
                    ExtractDirection::UserOwes => DIRECTION_USER_OWES,
                    ExtractDirection::OwedToUser => DIRECTION_OWED_TO_USER,
                    ExtractDirection::Unclear => "unclear",
                },
                expected_date,
                party_guess,
                now_iso(),
                provenance.as_str(),
                conf,
                entry_source_id
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_drafts(&self) -> rusqlite::Result<Vec<DraftCommitment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, description, direction, expected_date, party_guess, created_at,
                    source_provenance, confidence_json, entry_source_id
             FROM draft_commitment ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            let dir: String = r.get(2)?;
            Ok(DraftCommitment {
                id: r.get(0)?,
                description: r.get(1)?,
                direction: match dir.as_str() {
                    DIRECTION_USER_OWES => ExtractDirection::UserOwes,
                    DIRECTION_OWED_TO_USER => ExtractDirection::OwedToUser,
                    _ => ExtractDirection::Unclear,
                },
                expected_date: r.get(3)?,
                party_guess: r.get(4)?,
                created_at: r.get(5)?,
                source_provenance: provenance_from(r.get::<_, String>(6)?.as_str()),
                confidence_json: r.get(7)?,
                entry_source_id: r.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_draft(&self, id: i64) -> rusqlite::Result<usize> {
        self.conn
            .execute("DELETE FROM draft_commitment WHERE id=?1", params![id])
    }

    /// Confirm a draft into a real commitment, applying optional edits.
    pub fn confirm_draft(
        &self,
        draft_id: i64,
        description: Option<&str>,
        direction: Option<Direction>,
        expected_date: Option<Option<&str>>,
        party: Option<&str>,
    ) -> rusqlite::Result<Option<i64>> {
        let drafts = self.list_drafts()?;
        let d = match drafts.into_iter().find(|d| d.id == draft_id) {
            Some(d) => d,
            None => return Ok(None),
        };
        let dir = direction
            .map(|d| ExtractDirection::UserOwes.with_dir(d))
            .unwrap_or(d.direction);
        let firm = match dir {
            ExtractDirection::UserOwes => Direction::UserOwes,
            ExtractDirection::OwedToUser => Direction::OwedToUser,
            ExtractDirection::Unclear => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "cannot confirm a draft without a decided direction".into(),
                ))
            }
        };
        let desc = description.unwrap_or(&d.description);
        let date = match expected_date {
            Some(x) => x,
            None => d.expected_date.as_deref(),
        };
        let party = party.or(d.party_guess.as_deref());

        let (by, to) = match firm {
            Direction::UserOwes => (Some("user"), party),
            Direction::OwedToUser => (party, Some("user")),
        };

        let id = self.create_commitment(NewCommitment {
            description: desc,
            direction: firm,
            expected_date: date,
            owed_by_party: by,
            owed_to_party: to,
            provenance: d.source_provenance,
            confidence: serde_json::from_str(&d.confidence_json).unwrap_or_default(),
        })?;

        if let Some(src) = d.entry_source_id {
            self.conn.execute(
                "INSERT INTO edge_derived_from(commitment_id, entry_source_id) VALUES (?1,?2)",
                params![id, src],
            )?;
        }
        self.delete_draft(draft_id)?;
        Ok(Some(id))
    }
}

impl ExtractDirection {
    fn with_dir(self, d: Direction) -> ExtractDirection {
        match d {
            Direction::UserOwes => ExtractDirection::UserOwes,
            Direction::OwedToUser => ExtractDirection::OwedToUser,
        }
    }
}

pub fn provenance_from(s: &str) -> Provenance {
    match s {
        "model_extracted" => Provenance::ModelExtracted,
        "rule_extracted" => Provenance::RuleExtracted,
        _ => Provenance::Manual,
    }
}

/// Input for creating a commitment.
pub struct NewCommitment<'a> {
    pub description: &'a str,
    pub direction: Direction,
    pub expected_date: Option<&'a str>,
    pub owed_by_party: Option<&'a str>,
    pub owed_to_party: Option<&'a str>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

const GET_BASE: &str = r#"
SELECT c.id, c.description, c.direction, c.expected_date, c.status, c.created_at,
       c.last_updated_at, c.source_provenance, c.confidence_json,
       by_p.name AS owed_by_name, to_p.name AS owed_to_name
FROM commitment c
LEFT JOIN edge_owed_by eb ON eb.commitment_id = c.id
LEFT JOIN party by_p ON by_p.id = eb.party_id
LEFT JOIN edge_owed_to et ON et.commitment_id = c.id
LEFT JOIN party to_p ON to_p.id = et.party_id
"#;

const GET_ONE: &str = r#"
SELECT c.id, c.description, c.direction, c.expected_date, c.status, c.created_at,
       c.last_updated_at, c.source_provenance, c.confidence_json,
       by_p.name AS owed_by_name, to_p.name AS owed_to_name
FROM commitment c
LEFT JOIN edge_owed_by eb ON eb.commitment_id = c.id
LEFT JOIN party by_p ON by_p.id = eb.party_id
LEFT JOIN edge_owed_to et ON et.commitment_id = c.id
LEFT JOIN party to_p ON to_p.id = et.party_id
WHERE c.id = ?1
"#;

impl Store {
    fn row_to_commitment(r: &Row) -> rusqlite::Result<Commitment> {
        let dir: String = r.get(2)?;
        let st: String = r.get(4)?;
        Ok(Commitment {
            id: r.get(0)?,
            description: r.get(1)?,
            direction: if dir == DIRECTION_USER_OWES {
                Direction::UserOwes
            } else {
                Direction::OwedToUser
            },
            expected_date: r.get(3)?,
            status: match st.as_str() {
                "resolved" => Status::Resolved,
                "snoozed" => Status::Snoozed,
                "overdue" => Status::Overdue,
                _ => Status::Open,
            },
            created_at: r.get(5)?,
            last_updated_at: r.get(6)?,
            source_provenance: provenance_from(r.get::<_, String>(7)?.as_str()),
            confidence_json: r.get(8)?,
            owed_by: r.get(9)?,
            owed_to: r.get(10)?,
        })
    }
}
