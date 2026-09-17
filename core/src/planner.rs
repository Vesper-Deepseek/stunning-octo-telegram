//! Rule-based planner: decides what to surface to the user and when.
//! Deterministic, aging-driven; no ML.

use chrono::NaiveDate;

use crate::models::{Commitment, Status};

#[derive(Debug, Clone)]
pub enum PlanAction {
    /// show it in the main feed / fire a notification
    SurfaceNow,
    /// keep it out of the way until `until`
    Snooze { until: Option<NaiveDate> },
    /// overdue and aging — stronger reminder
    EscalateReminder,
    /// resolved long enough ago; hide from active views
    Archive,
}

#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// open undated commitments younger than this stay quiet
    pub quiet_days: i64,
    /// open undated commitments older than this surface
    pub surface_after_days: i64,
    /// days past expected_date before escalation kicks in
    pub escalate_after_overdue_days: i64,
    /// days after resolution before archiving
    pub archive_resolved_after_days: i64,
    /// dated commitments surface this many days before due date
    pub pre_due_surface_days: i64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            quiet_days: 2,
            surface_after_days: 3,
            escalate_after_overdue_days: 7,
            archive_resolved_after_days: 30,
            pre_due_surface_days: 1,
        }
    }
}

fn parse_iso(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn created_date(c: &Commitment) -> NaiveDate {
    // created_at is RFC3339; take the date part
    c.created_at
        .split('T')
        .next()
        .and_then(parse_iso)
        .unwrap_or_else(|| today_fallback())
}

fn updated_date(c: &Commitment) -> NaiveDate {
    c.last_updated_at
        .split('T')
        .next()
        .and_then(parse_iso)
        .unwrap_or_else(|| today_fallback())
}

fn today_fallback() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Decide what should happen to `c` on `today`.
pub fn plan(c: &Commitment, today: NaiveDate, cfg: &PlannerConfig) -> PlanAction {
    match c.status {
        Status::Resolved => {
            let age = (today - updated_date(c)).num_days();
            if age >= cfg.archive_resolved_after_days {
                PlanAction::Archive
            } else {
                PlanAction::Snooze { until: None }
            }
        }
        Status::Snoozed => PlanAction::Snooze { until: None },
        Status::Overdue => {
            let overdue_days = c
                .expected_date
                .as_deref()
                .and_then(parse_iso)
                .map(|d| (today - d).num_days())
                .unwrap_or(0);
            if overdue_days >= cfg.escalate_after_overdue_days {
                PlanAction::EscalateReminder
            } else {
                PlanAction::SurfaceNow
            }
        }
        Status::Open => match c.expected_date.as_deref().and_then(parse_iso) {
            Some(due) => {
                let days_until = (due - today).num_days();
                if days_until < 0 {
                    PlanAction::SurfaceNow
                } else if days_until <= cfg.pre_due_surface_days {
                    PlanAction::SurfaceNow
                } else {
                    PlanAction::Snooze { until: Some(due) }
                }
            }
            None => {
                let age = (today - created_date(c)).num_days();
                if age >= cfg.surface_after_days {
                    PlanAction::SurfaceNow
                } else if age <= cfg.quiet_days {
                    PlanAction::Snooze { until: None }
                } else {
                    PlanAction::Snooze { until: None }
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Direction, Provenance, Status};

    fn c_with(
        created_at: &str,
        last_updated_at: &str,
        expected_date: Option<&str>,
        status: Status,
    ) -> Commitment {
        Commitment {
            id: 0,
            description: "x".into(),
            direction: Direction::UserOwes,
            expected_date: expected_date.map(str::to_string),
            status,
            created_at: created_at.into(),
            last_updated_at: last_updated_at.into(),
            source_provenance: Provenance::Manual,
            confidence_json: "{}".into(),
            owed_by: None,
            owed_to: None,
        }
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn open_dated_overdue_surfaces_immediately() {
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-01T00:00:00+00:00",
            Some("2026-08-20"),
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::SurfaceNow
        ));
    }

    #[test]
    fn open_dated_at_threshold_surfaces() {
        // pre_due_surface_days default is 1 -> due == today + 1 should surface
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-01T00:00:00+00:00",
            Some("2026-08-27"),
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::SurfaceNow
        ));
    }

    #[test]
    fn open_dated_two_days_out_snoozes_to_due() {
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-01T00:00:00+00:00",
            Some("2026-08-28"),
            Status::Open,
        );
        let today = day(2026, 8, 26);
        match plan(&c, today, &PlannerConfig::default()) {
            PlanAction::Snooze { until: Some(d) } => assert_eq!(d, day(2026, 8, 28)),
            other => panic!("expected snooze-until-due, got {other:?}"),
        }
    }

    #[test]
    fn overdue_at_escalation_threshold_escalates() {
        // escalate_after_overdue_days default = 7
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-01T00:00:00+00:00",
            Some("2026-08-19"),
            Status::Overdue,
        );
        let today = day(2026, 8, 26); // 7 days past
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::EscalateReminder
        ));
    }

    #[test]
    fn overdue_below_escalation_threshold_surfaces() {
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-01T00:00:00+00:00",
            Some("2026-08-23"),
            Status::Overdue,
        );
        let today = day(2026, 8, 26); // 3 days past
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::SurfaceNow
        ));
    }

    #[test]
    fn open_undated_within_quiet_period_stays_quiet() {
        // age = 0, quiet_days default 2 -> Snooze
        let c = c_with(
            "2026-08-26T00:00:00+00:00",
            "2026-08-26T00:00:00+00:00",
            None,
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::Snooze { until: None }
        ));
    }

    #[test]
    fn open_undated_at_quiet_boundary_is_quiet() {
        // age = 2 == quiet_days default -> Snooze
        let c = c_with(
            "2026-08-24T00:00:00+00:00",
            "2026-08-24T00:00:00+00:00",
            None,
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::Snooze { until: None }
        ));
    }

    #[test]
    fn open_undated_old_surfaces() {
        // age = 30, surface_after_days default 3 -> Surface
        let c = c_with(
            "2026-07-27T00:00:00+00:00",
            "2026-07-27T00:00:00+00:00",
            None,
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::SurfaceNow
        ));
    }

    #[test]
    fn open_undated_mid_window_stays_quiet() {
        // Spec intent: between quiet window and surface threshold, the planner
        // should still snooze (the variable is named `quiet_days`).
        // With cfg quiet=2, surface=10, an age of 5 should Snooze.
        let cfg = PlannerConfig {
            quiet_days: 2,
            surface_after_days: 10,
            ..PlannerConfig::default()
        };
        let c = c_with(
            "2026-08-21T00:00:00+00:00",
            "2026-08-21T00:00:00+00:00",
            None,
            Status::Open,
        );
        let today = day(2026, 8, 26);
        assert!(
            matches!(plan(&c, today, &cfg), PlanAction::Snooze { until: None }),
            "mid-window undated commitment should remain quiet, not surface"
        );
    }

    #[test]
    fn snoozed_always_snoozes() {
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-26T00:00:00+00:00",
            None,
            Status::Snoozed,
        );
        let today = day(2026, 8, 26);
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::Snooze { until: None }
        ));
    }

    #[test]
    fn resolved_freshly_does_not_archive() {
        let c = c_with(
            "2026-08-01T00:00:00+00:00",
            "2026-08-25T00:00:00+00:00",
            None,
            Status::Resolved,
        );
        let today = day(2026, 8, 26);
        // 1 day after update; archive threshold 30 -> Snooze, not Archive
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::Snooze { until: None }
        ));
    }

    #[test]
    fn resolved_at_archive_boundary_archives() {
        // archive_resolved_after_days default 30 -> age 30 == threshold => Archive
        let c = c_with(
            "2026-07-01T00:00:00+00:00",
            "2026-07-27T00:00:00+00:00",
            None,
            Status::Resolved,
        );
        let today = day(2026, 8, 26); // 30 days after last update
        assert!(matches!(
            plan(&c, today, &PlannerConfig::default()),
            PlanAction::Archive
        ));
    }
}
