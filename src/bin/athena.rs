use athena_v2::feedback::TaskOutcome;
use athena_v2::ids::{PacketId, PurposeId};
use athena_v2::persisted::{
    FeedbackApplyInput, apply_feedback_command, create_purpose, update_purpose,
};
use athena_v2::storage::DoltStorage;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

struct Config {
    db_path: PathBuf,
    fixture_path: PathBuf,
    command: CommandKind,
}

enum CommandKind {
    PurposeCreate {
        statement: String,
        success_criteria: String,
    },
    PurposeUpdate {
        purpose_id: PurposeId,
        statement: String,
        success_criteria: String,
    },
    FeedbackApply {
        purpose_id: PurposeId,
        packet_id: PacketId,
        outcome: TaskOutcome,
    },
}

fn default_db_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".athena/db")
}

fn default_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".athena/fragments.json")
}

fn usage() -> &'static str {
    "usage:
  athena [--db path] [--fixture path] purpose create --statement <text> --success-criteria <text>
  athena [--db path] [--fixture path] purpose update --purpose-id <id> --statement <text> --success-criteria <text>
  athena [--db path] [--fixture path] feedback apply --purpose-id <id> --packet-id <id> --outcome <success|partial|failed> < feedback.json"
}

fn require_flag(args: &[String], index: &mut usize, flag: &str) -> Result<String, io::Error> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| io::Error::other(format!("{flag} requires value")))
}

fn parse_outcome(value: &str) -> Result<TaskOutcome, io::Error> {
    match value {
        "success" => Ok(TaskOutcome::Success),
        "partial" => Ok(TaskOutcome::Partial),
        "failed" => Ok(TaskOutcome::Failed),
        other => Err(io::Error::other(format!("invalid outcome: {other}"))),
    }
}

fn parse_args() -> Result<Config, io::Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err(io::Error::other(usage()));
    }

    let mut db_path = default_db_path();
    let mut fixture_path = default_fixture_path();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--db" => {
                db_path = PathBuf::from(require_flag(&args, &mut index, "--db")?);
            }
            "--fixture" => {
                fixture_path = PathBuf::from(require_flag(&args, &mut index, "--fixture")?);
            }
            "purpose" => {
                index += 1;
                let action = args
                    .get(index)
                    .ok_or_else(|| io::Error::other("purpose requires action"))?;
                return match action.as_str() {
                    "create" => {
                        let mut statement = None;
                        let mut success_criteria = None;
                        index += 1;
                        while index < args.len() {
                            match args[index].as_str() {
                                "--statement" => {
                                    statement =
                                        Some(require_flag(&args, &mut index, "--statement")?);
                                }
                                "--success-criteria" => {
                                    success_criteria = Some(require_flag(
                                        &args,
                                        &mut index,
                                        "--success-criteria",
                                    )?);
                                }
                                other => {
                                    return Err(io::Error::other(format!(
                                        "unknown argument: {other}"
                                    )));
                                }
                            }
                            index += 1;
                        }
                        Ok(Config {
                            db_path,
                            fixture_path,
                            command: CommandKind::PurposeCreate {
                                statement: statement
                                    .ok_or_else(|| io::Error::other("--statement is required"))?,
                                success_criteria: success_criteria.ok_or_else(|| {
                                    io::Error::other("--success-criteria is required")
                                })?,
                            },
                        })
                    }
                    "update" => {
                        let mut purpose_id = None;
                        let mut statement = None;
                        let mut success_criteria = None;
                        index += 1;
                        while index < args.len() {
                            match args[index].as_str() {
                                "--purpose-id" => {
                                    purpose_id = Some(PurposeId::new(require_flag(
                                        &args,
                                        &mut index,
                                        "--purpose-id",
                                    )?));
                                }
                                "--statement" => {
                                    statement =
                                        Some(require_flag(&args, &mut index, "--statement")?);
                                }
                                "--success-criteria" => {
                                    success_criteria = Some(require_flag(
                                        &args,
                                        &mut index,
                                        "--success-criteria",
                                    )?);
                                }
                                other => {
                                    return Err(io::Error::other(format!(
                                        "unknown argument: {other}"
                                    )));
                                }
                            }
                            index += 1;
                        }
                        Ok(Config {
                            db_path,
                            fixture_path,
                            command: CommandKind::PurposeUpdate {
                                purpose_id: purpose_id
                                    .ok_or_else(|| io::Error::other("--purpose-id is required"))?,
                                statement: statement
                                    .ok_or_else(|| io::Error::other("--statement is required"))?,
                                success_criteria: success_criteria.ok_or_else(|| {
                                    io::Error::other("--success-criteria is required")
                                })?,
                            },
                        })
                    }
                    other => Err(io::Error::other(format!("unknown purpose action: {other}"))),
                };
            }
            "feedback" => {
                index += 1;
                let action = args
                    .get(index)
                    .ok_or_else(|| io::Error::other("feedback requires action"))?;
                if action != "apply" {
                    return Err(io::Error::other(format!(
                        "unknown feedback action: {action}"
                    )));
                }

                let mut purpose_id = None;
                let mut packet_id = None;
                let mut outcome = None;
                index += 1;
                while index < args.len() {
                    match args[index].as_str() {
                        "--purpose-id" => {
                            purpose_id = Some(PurposeId::new(require_flag(
                                &args,
                                &mut index,
                                "--purpose-id",
                            )?));
                        }
                        "--packet-id" => {
                            packet_id = Some(PacketId::new(require_flag(
                                &args,
                                &mut index,
                                "--packet-id",
                            )?));
                        }
                        "--outcome" => {
                            outcome = Some(parse_outcome(&require_flag(
                                &args,
                                &mut index,
                                "--outcome",
                            )?)?);
                        }
                        other => {
                            return Err(io::Error::other(format!("unknown argument: {other}")));
                        }
                    }
                    index += 1;
                }
                return Ok(Config {
                    db_path,
                    fixture_path,
                    command: CommandKind::FeedbackApply {
                        purpose_id: purpose_id
                            .ok_or_else(|| io::Error::other("--purpose-id is required"))?,
                        packet_id: packet_id
                            .ok_or_else(|| io::Error::other("--packet-id is required"))?,
                        outcome: outcome
                            .ok_or_else(|| io::Error::other("--outcome is required"))?,
                    },
                });
            }
            "--help" | "-h" => return Err(io::Error::other(usage())),
            other => return Err(io::Error::other(format!("unknown argument: {other}"))),
        }

        index += 1;
    }

    Err(io::Error::other(usage()))
}

fn read_feedback_input() -> Result<FeedbackApplyInput, io::Error> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        return Err(io::Error::other("stdin request body was empty"));
    }
    serde_json::from_str(&raw).map_err(io::Error::other)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_args()?;
    let storage = DoltStorage::open(&config.db_path)?;

    match config.command {
        CommandKind::PurposeCreate {
            statement,
            success_criteria,
        } => {
            let result = create_purpose(
                &storage,
                &config.fixture_path,
                &statement,
                &success_criteria,
            )?;
            serde_json::to_writer_pretty(io::stdout(), &result)?;
        }
        CommandKind::PurposeUpdate {
            purpose_id,
            statement,
            success_criteria,
        } => {
            let result = update_purpose(
                &storage,
                &config.fixture_path,
                &purpose_id,
                &statement,
                &success_criteria,
            )?;
            serde_json::to_writer_pretty(io::stdout(), &result)?;
        }
        CommandKind::FeedbackApply {
            purpose_id,
            packet_id,
            outcome,
        } => {
            let input = read_feedback_input()?;
            let result = apply_feedback_command(
                &storage,
                &config.fixture_path,
                &purpose_id,
                &packet_id,
                outcome,
                input,
            )?;
            serde_json::to_writer_pretty(io::stdout(), &result)?;
        }
    }

    println!();
    Ok(())
}
