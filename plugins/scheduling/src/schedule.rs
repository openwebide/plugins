//! Cron/timezone validation and recurrence are plugin behavior, not host callbacks.
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSelection {
    pub server_id: i64,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    Once {
        at: i64,
    },
    Cron {
        expression: String,
        timezone: String,
    },
}
impl Schedule {
    pub fn next_after(&self, after: i64) -> Result<Option<i64>, String> {
        match self {
            Self::Once { at } => Ok((*at > after).then_some(*at)),
            Self::Cron {
                expression,
                timezone,
            } => {
                if expression.len() > 128 || expression.split_whitespace().count() != 5 {
                    return Err(
                        "Use a five-field cron expression (minute hour day month weekday).".into(),
                    );
                }
                let zone = Tz::from_str(timezone).map_err(|_| "Choose a valid timezone.")?;
                let start = DateTime::<Utc>::from_timestamp(after, 0)
                    .ok_or("Invalid schedule timestamp.")?
                    .with_timezone(&zone);
                let cron = croner::Cron::from_str(expression)
                    .map_err(|error| format!("Invalid schedule: {error}"))?;
                let next = cron
                    .find_next_occurrence(&start, false)
                    .map_err(|error| format!("Schedule has no next occurrence: {error}"))?;
                Ok(Some(next.timestamp()))
            }
        }
    }
    pub fn validate(&self, now: i64) -> Result<(), String> {
        if self.next_after(now)?.is_none() {
            return Err("Choose a future date and time.".into());
        }
        Ok(())
    }
}
/// Session policy is resolved when an occurrence becomes due, not when saved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTarget {
    #[default]
    Existing,
    New,
    Latest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    #[serde(default)]
    pub session_target: SessionTarget,
    #[serde(default)]
    pub auto_title: bool,
    #[serde(default)]
    pub title: String,
    pub prompt: String,
    #[serde(default)]
    pub session_id: i64,
    /// None resolves the session’s current model when the task runs.
    #[serde(default)]
    pub model: Option<ModelSelection>,
    pub schedule: Schedule,
    pub enabled: bool,
}
impl Draft {
    pub fn validate(&self, now: i64) -> Result<(), String> {
        if (!self.auto_title && self.title.trim().is_empty()) || self.title.chars().count() > 120 {
            return Err("Task titles need 1–120 characters.".into());
        }
        if self.prompt.trim().is_empty() || self.prompt.len() > 32 * 1024 {
            return Err("Task prompts need 1–32 KiB of text.".into());
        }
        if self.session_target == SessionTarget::Existing && self.session_id <= 0 {
            return Err("Choose a session for this task.".into());
        }
        if let Some(model) = &self.model
            && (model.server_id <= 0
                || model.model.trim().is_empty()
                || model.model.len() > 256
                || model.model.chars().any(char::is_control))
        {
            return Err("Choose a valid server and model for this task.".into());
        }
        self.schedule.validate(now)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorStart {
    pub prompt: String,
    pub delay_seconds: i64,
    #[serde(default = "interval")]
    pub interval_seconds: i64,
    #[serde(default = "checks")]
    pub max_checks: i64,
}
const fn interval() -> i64 {
    600
}
const fn checks() -> i64 {
    1
}
impl MonitorStart {
    pub fn draft(&self, session: i64, now: i64) -> Result<(Draft, i64), String> {
        if !(5..86400).contains(&self.delay_seconds)
            || !(5..=86400).contains(&self.interval_seconds)
            || !(1..=24).contains(&self.max_checks)
            || self
                .delay_seconds
                .saturating_add(self.interval_seconds.saturating_mul(self.max_checks - 1))
                >= 86400
        {
            return Err("Monitor checks must fit within 24 hours with delays of 5–86399 seconds, intervals of 5–86400 seconds and 1–24 checks".into());
        }
        if now < 0 {
            return Err("Invalid clock".into());
        }
        let expires = now.checked_add(86400).ok_or("Invalid expiry")?;
        let draft = Draft {
            session_target: SessionTarget::Existing,
            auto_title: false,
            title: "Monitor".into(),
            prompt: self.prompt.clone(),
            session_id: session,
            model: None,
            schedule: Schedule::Once {
                at: now
                    .checked_add(self.delay_seconds)
                    .ok_or("Invalid monitor timestamp")?,
            },
            enabled: true,
        };
        draft.validate(now)?;
        Ok((draft, expires))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stamp(value: &str) -> i64 {
        DateTime::parse_from_rfc3339(value).unwrap().timestamp()
    }
    #[test]
    fn recurrence_preserves_wall_time_across_spring_and_fall_dst() {
        let schedule = Schedule::Cron {
            expression: "0 9 * * *".into(),
            timezone: "America/Chicago".into(),
        };
        assert_eq!(
            schedule.next_after(stamp("2026-03-07T16:00:00Z")).unwrap(),
            Some(stamp("2026-03-08T14:00:00Z"))
        );
        assert_eq!(
            schedule.next_after(stamp("2026-10-31T15:00:00Z")).unwrap(),
            Some(stamp("2026-11-01T15:00:00Z"))
        );
    }
    #[test]
    fn once_is_future_only_and_cron_rejects_invalid_fields_and_timezones() {
        assert_eq!(Schedule::Once { at: 100 }.next_after(100).unwrap(), None);
        assert!(Schedule::Once { at: 100 }.validate(100).is_err());
        for (expression, timezone) in [
            ("* * * * * *", "UTC"),
            ("0 9 * * *", "Mars/Base"),
            ("99 9 * * *", "UTC"),
        ] {
            assert!(
                Schedule::Cron {
                    expression: expression.into(),
                    timezone: timezone.into()
                }
                .validate(0)
                .is_err()
            );
        }
    }
    #[test]
    fn monitor_limits_prevent_deadline_extension_and_timestamp_overflow() {
        let mut start = MonitorStart {
            prompt: "Check build".into(),
            delay_seconds: 5,
            interval_seconds: 600,
            max_checks: 24,
        };
        let (draft, expiry) = start.draft(1, 100).unwrap();
        assert_eq!(expiry, 86500);
        assert_eq!(draft.schedule, Schedule::Once { at: 105 });
        for (delay, interval, checks) in [
            (4, 600, 1),
            (86400, 600, 1),
            (5, 4, 1),
            (5, 600, 25),
            (86399, 600, 2),
        ] {
            start.delay_seconds = delay;
            start.interval_seconds = interval;
            start.max_checks = checks;
            assert!(start.draft(1, 100).is_err());
        }
        start.delay_seconds = 5;
        start.interval_seconds = 600;
        start.max_checks = 1;
        assert!(start.draft(1, i64::MAX).is_err());
        assert!(start.draft(0, 0).is_err());
    }
    #[test]
    fn task_validation_keeps_model_and_prompt_policy_in_source() {
        let mut draft = Draft {
            session_target: SessionTarget::New,
            auto_title: true,
            title: String::new(),
            prompt: "Do work".into(),
            session_id: 0,
            model: None,
            schedule: Schedule::Once { at: 100 },
            enabled: true,
        };
        draft.validate(0).unwrap();
        draft.model = Some(ModelSelection {
            server_id: 0,
            model: "model".into(),
        });
        assert!(draft.validate(0).is_err());
        draft.model = None;
        draft.auto_title = false;
        assert!(draft.validate(0).is_err());
        draft.title = "Task".into();
        draft.prompt = "x".repeat(32 * 1024 + 1);
        assert!(draft.validate(0).is_err());
    }
}
