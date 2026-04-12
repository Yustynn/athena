use crate::error::AthenaError;
use crate::feedback::{FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome};
use crate::fragment::{Fragment, FragmentKind};
use crate::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use crate::packet::PurposePacket;
use crate::purpose::{Purpose, PurposeStatus};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DoltStorage {
    repo_path: PathBuf,
    home_dir: PathBuf,
}

impl DoltStorage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AthenaError> {
        let repo_path = path.as_ref().to_path_buf();
        let home_dir = repo_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".dolt-home");
        let storage = Self {
            repo_path,
            home_dir,
        };
        storage.ensure_repo()?;
        storage.init_schema()?;
        Ok(storage)
    }

    pub fn commit_all(&self, message: &str) -> Result<bool, AthenaError> {
        if !self.has_uncommitted_changes()? {
            return Ok(false);
        }

        self.run_command(&["add", "."])?;
        self.run_command(&["commit", "-m", message])?;
        Ok(true)
    }

    fn ensure_repo(&self) -> Result<(), AthenaError> {
        fs::create_dir_all(&self.repo_path)?;
        fs::create_dir_all(&self.home_dir)?;

        if self.repo_path.join(".dolt").exists() {
            return Ok(());
        }

        self.run_command(&["init", "--name", "Athena", "--email", "athena@local"])?;
        Ok(())
    }

    fn init_schema(&self) -> Result<(), AthenaError> {
        self.exec(
            "
            CREATE TABLE IF NOT EXISTS purposes (
                purpose_id VARCHAR(255) PRIMARY KEY,
                statement TEXT NOT NULL,
                success_criteria TEXT NOT NULL,
                status VARCHAR(32) NOT NULL
            );

            CREATE TABLE IF NOT EXISTS packets (
                packet_id VARCHAR(255) PRIMARY KEY,
                purpose_id VARCHAR(255) NOT NULL
            );

            CREATE TABLE IF NOT EXISTS packet_fragments (
                packet_id VARCHAR(255) NOT NULL,
                fragment_id VARCHAR(255) NOT NULL,
                kind VARCHAR(32) NOT NULL,
                text TEXT NOT NULL,
                position BIGINT NOT NULL,
                PRIMARY KEY (packet_id, fragment_id)
            );

            CREATE TABLE IF NOT EXISTS feedback_events (
                feedback_id VARCHAR(255) PRIMARY KEY,
                purpose_id VARCHAR(255) NOT NULL,
                packet_id VARCHAR(255) NOT NULL,
                outcome VARCHAR(32) NOT NULL
            );

            CREATE TABLE IF NOT EXISTS feedback_fragments (
                feedback_id VARCHAR(255) NOT NULL,
                fragment_id VARCHAR(255) NOT NULL,
                verdict VARCHAR(32) NOT NULL,
                reason TEXT,
                position BIGINT NOT NULL,
                PRIMARY KEY (feedback_id, fragment_id)
            );

            CREATE TABLE IF NOT EXISTS fragment_nodes (
                fragment_id VARCHAR(255) PRIMARY KEY,
                kind VARCHAR(32) NOT NULL,
                text TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS fragment_edges (
                from_fragment_id VARCHAR(255) NOT NULL,
                to_fragment_id VARCHAR(255) NOT NULL,
                edge_type VARCHAR(255) NOT NULL,
                PRIMARY KEY (from_fragment_id, to_fragment_id, edge_type)
            );
            ",
        )?;
        Ok(())
    }

    pub fn insert_purpose(&self, purpose: &Purpose) -> Result<(), AthenaError> {
        self.exec(&format!(
            "INSERT INTO purposes (purpose_id, statement, success_criteria, status)
             VALUES ({}, {}, {}, {})
             ON DUPLICATE KEY UPDATE
                statement = VALUES(statement),
                success_criteria = VALUES(success_criteria),
                status = VALUES(status);",
            sql_str(&purpose.purpose_id.0),
            sql_str(&purpose.statement),
            sql_str(&purpose.success_criteria),
            sql_str(purpose_status_to_db(&purpose.status)),
        ))
    }

    pub fn get_purpose(&self, purpose_id: &PurposeId) -> Result<Option<Purpose>, AthenaError> {
        let rows = self.query_json_rows(&format!(
            "SELECT purpose_id, statement, success_criteria, status FROM purposes WHERE purpose_id = {};",
            sql_str(&purpose_id.0)
        ))?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };

        Ok(Some(Purpose {
            purpose_id: PurposeId::new(json_str(row, "purpose_id")?),
            statement: json_str(row, "statement")?,
            success_criteria: json_str(row, "success_criteria")?,
            status: purpose_status_from_db(&json_str(row, "status")?)?,
        }))
    }

    pub fn insert_packet(&self, packet: &PurposePacket) -> Result<(), AthenaError> {
        self.exec(&format!(
            "INSERT INTO packets (packet_id, purpose_id)
             VALUES ({}, {})
             ON DUPLICATE KEY UPDATE purpose_id = VALUES(purpose_id);",
            sql_str(&packet.packet_id.0),
            sql_str(&packet.purpose_id.0),
        ))?;

        self.exec(&format!(
            "DELETE FROM packet_fragments WHERE packet_id = {};",
            sql_str(&packet.packet_id.0),
        ))?;

        for (position, fragment) in packet.fragments.iter().enumerate() {
            self.exec(&format!(
                "INSERT INTO packet_fragments (packet_id, fragment_id, kind, text, position)
                 VALUES ({}, {}, {}, {}, {});",
                sql_str(&packet.packet_id.0),
                sql_str(&fragment.fragment_id.0),
                sql_str(fragment_kind_to_db(&fragment.kind)),
                sql_str(&fragment.text),
                position,
            ))?;
        }
        Ok(())
    }

    pub fn get_packet(&self, packet_id: &PacketId) -> Result<Option<PurposePacket>, AthenaError> {
        let packet_rows = self.query_json_rows(&format!(
            "SELECT purpose_id FROM packets WHERE packet_id = {};",
            sql_str(&packet_id.0)
        ))?;

        let Some(packet_row) = packet_rows.first() else {
            return Ok(None);
        };

        let purpose_id = PurposeId::new(json_str(packet_row, "purpose_id")?);
        let fragment_rows = self.query_json_rows(&format!(
            "SELECT fragment_id, kind, text FROM packet_fragments WHERE packet_id = {} ORDER BY position ASC;",
            sql_str(&packet_id.0)
        ))?;

        let fragments = fragment_rows
            .into_iter()
            .map(|row| {
                Ok(Fragment {
                    fragment_id: FragmentId::new(json_str(&row, "fragment_id")?),
                    kind: fragment_kind_from_db(&json_str(&row, "kind")?)?,
                    text: json_str(&row, "text")?,
                })
            })
            .collect::<Result<Vec<_>, AthenaError>>()?;

        Ok(Some(PurposePacket {
            packet_id: packet_id.clone(),
            purpose_id,
            fragments,
        }))
    }

    pub fn insert_feedback(&self, feedback: &FeedbackEvent) -> Result<(), AthenaError> {
        self.exec(&format!(
            "INSERT INTO feedback_events (feedback_id, purpose_id, packet_id, outcome)
             VALUES ({}, {}, {}, {})
             ON DUPLICATE KEY UPDATE
                purpose_id = VALUES(purpose_id),
                packet_id = VALUES(packet_id),
                outcome = VALUES(outcome);",
            sql_str(&feedback.feedback_id.0),
            sql_str(&feedback.purpose_id.0),
            sql_str(&feedback.packet_id.0),
            sql_str(task_outcome_to_db(&feedback.outcome)),
        ))?;

        self.exec(&format!(
            "DELETE FROM feedback_fragments WHERE feedback_id = {};",
            sql_str(&feedback.feedback_id.0),
        ))?;

        for (position, fragment_feedback) in feedback.fragment_feedback.iter().enumerate() {
            let reason = fragment_feedback
                .reason
                .as_ref()
                .map_or_else(|| "NULL".to_string(), |value| sql_str(value));

            self.exec(&format!(
                "INSERT INTO feedback_fragments (feedback_id, fragment_id, verdict, reason, position)
                 VALUES ({}, {}, {}, {}, {});",
                sql_str(&feedback.feedback_id.0),
                sql_str(&fragment_feedback.fragment_id.0),
                sql_str(fragment_verdict_to_db(&fragment_feedback.verdict)),
                reason,
                position,
            ))?;
        }
        Ok(())
    }

    pub fn get_feedback(
        &self,
        feedback_id: &FeedbackId,
    ) -> Result<Option<FeedbackEvent>, AthenaError> {
        let rows = self.query_json_rows(&format!(
            "SELECT purpose_id, packet_id, outcome FROM feedback_events WHERE feedback_id = {};",
            sql_str(&feedback_id.0)
        ))?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let fragment_rows = self.query_json_rows(&format!(
            "SELECT fragment_id, verdict, IFNULL(reason, '') AS reason
             FROM feedback_fragments
             WHERE feedback_id = {}
             ORDER BY position ASC;",
            sql_str(&feedback_id.0)
        ))?;

        let fragment_feedback = fragment_rows
            .into_iter()
            .map(|row| {
                let reason = json_str(&row, "reason")?;
                Ok(FragmentFeedback {
                    fragment_id: FragmentId::new(json_str(&row, "fragment_id")?),
                    verdict: fragment_verdict_from_db(&json_str(&row, "verdict")?)?,
                    reason: if reason.is_empty() {
                        None
                    } else {
                        Some(reason)
                    },
                })
            })
            .collect::<Result<Vec<_>, AthenaError>>()?;

        Ok(Some(FeedbackEvent {
            feedback_id: feedback_id.clone(),
            purpose_id: PurposeId::new(json_str(row, "purpose_id")?),
            packet_id: PacketId::new(json_str(row, "packet_id")?),
            outcome: task_outcome_from_db(&json_str(row, "outcome")?)?,
            fragment_feedback,
        }))
    }

    pub fn insert_fragment_node(
        &self,
        fragment_id: &FragmentId,
        kind: &FragmentKind,
        text: &str,
    ) -> Result<(), AthenaError> {
        self.exec(&format!(
            "INSERT INTO fragment_nodes (fragment_id, kind, text) VALUES ({}, {}, {});",
            sql_str(&fragment_id.0),
            sql_str(fragment_kind_to_db(kind)),
            sql_str(text),
        ))
    }

    pub fn insert_fragment_edge(
        &self,
        from_fragment_id: &FragmentId,
        to_fragment_id: &FragmentId,
        edge_type: &str,
    ) -> Result<(), AthenaError> {
        self.exec(&format!(
            "INSERT INTO fragment_edges (from_fragment_id, to_fragment_id, edge_type) VALUES ({}, {}, {});",
            sql_str(&from_fragment_id.0),
            sql_str(&to_fragment_id.0),
            sql_str(edge_type),
        ))
    }

    pub fn outgoing_edges(
        &self,
        from_fragment_id: &FragmentId,
    ) -> Result<Vec<(FragmentId, String)>, AthenaError> {
        let rows = self.query_json_rows(&format!(
            "SELECT to_fragment_id, edge_type FROM fragment_edges WHERE from_fragment_id = {} ORDER BY to_fragment_id ASC, edge_type ASC;",
            sql_str(&from_fragment_id.0)
        ))?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    FragmentId::new(json_str(&row, "to_fragment_id")?),
                    json_str(&row, "edge_type")?,
                ))
            })
            .collect::<Result<Vec<_>, AthenaError>>()
    }

    fn has_uncommitted_changes(&self) -> Result<bool, AthenaError> {
        Ok(!self
            .query_json_rows("SELECT table_name FROM dolt_status LIMIT 1;")?
            .is_empty())
    }

    fn exec(&self, sql: &str) -> Result<(), AthenaError> {
        self.run_sql_command(sql, false).map(|_| ())
    }

    fn query_json_rows(&self, sql: &str) -> Result<Vec<Value>, AthenaError> {
        self.run_sql_command(sql, true)
    }

    fn run_sql_command(&self, sql: &str, json: bool) -> Result<Vec<Value>, AthenaError> {
        let mut args = vec!["sql", "-q", sql];
        if json {
            args.push("-r");
            args.push("json");
        }

        let output = self.run_command(&args)?;
        if !json {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let rows = serde_json::from_str::<Value>(&stdout)
            .ok()
            .and_then(|value| value.get("rows").cloned())
            .and_then(|rows| rows.as_array().cloned())
            .ok_or_else(|| {
                AthenaError::Io(std::io::Error::other(format!(
                    "failed to parse dolt json output: raw={stdout}"
                )))
            })?;
        Ok(rows)
    }

    fn run_command(&self, args: &[&str]) -> Result<std::process::Output, AthenaError> {
        let output = Command::new("dolt")
            .args(args)
            .current_dir(&self.repo_path)
            .env("HOME", &self.home_dir)
            .output()?;

        if output.status.success() {
            return Ok(output);
        }

        Err(AthenaError::Io(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )))
    }
}

fn sql_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn purpose_status_to_db(status: &PurposeStatus) -> &'static str {
    match status {
        PurposeStatus::Active => "active",
        PurposeStatus::Completed => "completed",
        PurposeStatus::Abandoned => "abandoned",
    }
}

fn purpose_status_from_db(value: &str) -> Result<PurposeStatus, AthenaError> {
    match value {
        "active" => Ok(PurposeStatus::Active),
        "completed" => Ok(PurposeStatus::Completed),
        "abandoned" => Ok(PurposeStatus::Abandoned),
        other => Err(AthenaError::Io(std::io::Error::other(format!(
            "invalid purpose status: {other}"
        )))),
    }
}

fn fragment_kind_to_db(kind: &FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Doctrine => "doctrine",
        FragmentKind::Procedure => "procedure",
        FragmentKind::Pitfall => "pitfall",
        FragmentKind::Context => "context",
    }
}

fn fragment_kind_from_db(value: &str) -> Result<FragmentKind, AthenaError> {
    match value {
        "doctrine" => Ok(FragmentKind::Doctrine),
        "procedure" => Ok(FragmentKind::Procedure),
        "pitfall" => Ok(FragmentKind::Pitfall),
        "context" => Ok(FragmentKind::Context),
        other => Err(AthenaError::Io(std::io::Error::other(format!(
            "invalid fragment kind: {other}"
        )))),
    }
}

fn task_outcome_to_db(outcome: &TaskOutcome) -> &'static str {
    match outcome {
        TaskOutcome::Success => "success",
        TaskOutcome::Partial => "partial",
        TaskOutcome::Failed => "failed",
    }
}

fn task_outcome_from_db(value: &str) -> Result<TaskOutcome, AthenaError> {
    match value {
        "success" => Ok(TaskOutcome::Success),
        "partial" => Ok(TaskOutcome::Partial),
        "failed" => Ok(TaskOutcome::Failed),
        other => Err(AthenaError::Io(std::io::Error::other(format!(
            "invalid task outcome: {other}"
        )))),
    }
}

fn fragment_verdict_to_db(verdict: &FragmentVerdict) -> &'static str {
    match verdict {
        FragmentVerdict::Helped => "helped",
        FragmentVerdict::Neutral => "neutral",
        FragmentVerdict::Wrong => "wrong",
    }
}

fn fragment_verdict_from_db(value: &str) -> Result<FragmentVerdict, AthenaError> {
    match value {
        "helped" => Ok(FragmentVerdict::Helped),
        "neutral" => Ok(FragmentVerdict::Neutral),
        "wrong" => Ok(FragmentVerdict::Wrong),
        other => Err(AthenaError::Io(std::io::Error::other(format!(
            "invalid fragment verdict: {other}"
        )))),
    }
}

fn json_str(value: &Value, key: &str) -> Result<String, AthenaError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AthenaError::Io(std::io::Error::other(format!("missing json key: {key}"))))
}
