use athena_v2::feedback::TaskOutcome;
use athena_v2::ids::{PacketId, PurposeId};
use athena_v2::persisted::{
    FeedbackApplyInput, apply_feedback_command, create_purpose, update_purpose,
};
use athena_v2::protocol::{AthenaRequest, handle_request};
use athena_v2::storage::DoltStorage;
use serde_json::{Value, json};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Stable,
    Dev,
}

fn stable_tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn dev_tool(name: &str, description: &str, input_schema: Value) -> Value {
    stable_tool(name, description, input_schema)
}

fn stable_tools() -> Vec<Value> {
    vec![
        stable_tool(
            "athena_get_latest_state",
            "Read latest persisted Athena purpose and packet from Dolt.",
            json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" }
                },
                "required": ["db_path"]
            }),
        ),
        stable_tool(
            "athena_create_purpose",
            "Create persisted Athena purpose and first packet.",
            json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" },
                    "statement": { "type": "string" },
                    "success_criteria": { "type": "string" }
                },
                "required": ["db_path", "statement", "success_criteria"]
            }),
        ),
        stable_tool(
            "athena_update_purpose",
            "Update persisted Athena purpose and reassemble packet.",
            json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" },
                    "purpose_id": { "type": "string" },
                    "statement": { "type": "string" },
                    "success_criteria": { "type": "string" }
                },
                "required": ["db_path", "purpose_id", "statement", "success_criteria"]
            }),
        ),
        stable_tool(
            "athena_apply_feedback",
            "Apply persisted Athena feedback, optionally create fragments, and assemble next packet.",
            json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" },
                    "purpose_id": { "type": "string" },
                    "packet_id": { "type": "string" },
                    "outcome": { "type": "string", "enum": ["success", "partial", "failed"] },
                    "fragment_feedback": { "type": "array" },
                    "new_fragments": { "type": "array" }
                },
                "required": ["db_path", "purpose_id", "packet_id", "outcome", "fragment_feedback"]
            }),
        ),
    ]
}

fn dev_tools() -> Vec<Value> {
    vec![
        dev_tool(
            "athena_dev_assemble_packet",
            "Assemble stateless Athena packet without persistence.",
            json!({
                "type": "object",
                "properties": {
                    "fixture_path": { "type": "string" },
                    "prompt": { "type": "string" },
                    "success_criteria": { "type": "string" }
                },
                "required": ["fixture_path", "prompt", "success_criteria"]
            }),
        ),
        dev_tool(
            "athena_dev_check_orientation",
            "Check orientation against a packet without persistence.",
            json!({
                "type": "object",
                "properties": {
                    "fixture_path": { "type": "string" },
                    "purpose": { "type": "object" },
                    "packet": { "type": "object" },
                    "response": { "type": "object" }
                },
                "required": ["fixture_path", "purpose", "packet", "response"]
            }),
        ),
        dev_tool(
            "athena_dev_apply_feedback",
            "Apply stateless feedback preview without persistence.",
            json!({
                "type": "object",
                "properties": {
                    "fixture_path": { "type": "string" },
                    "purpose": { "type": "object" },
                    "packet": { "type": "object" },
                    "feedback": { "type": "object" }
                },
                "required": ["fixture_path", "purpose", "packet", "feedback"]
            }),
        ),
    ]
}

fn parse_mode() -> Result<Mode, io::Error> {
    let mode = env::args()
        .nth(1)
        .ok_or_else(|| io::Error::other("usage: athena-mcp <stable|dev>"))?;

    match mode.as_str() {
        "stable" => Ok(Mode::Stable),
        "dev" => Ok(Mode::Dev),
        other => Err(io::Error::other(format!("unknown mode: {other}"))),
    }
}

fn parse_outcome(value: &str) -> Result<TaskOutcome, io::Error> {
    match value {
        "success" => Ok(TaskOutcome::Success),
        "partial" => Ok(TaskOutcome::Partial),
        "failed" => Ok(TaskOutcome::Failed),
        other => Err(io::Error::other(format!("invalid outcome: {other}"))),
    }
}

fn required_string(arguments: &Value, key: &str) -> Result<String, io::Error> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| io::Error::other(format!("missing string argument: {key}")))
}

fn text_result(value: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value).unwrap()
            }
        ],
        "structuredContent": value,
        "isError": false
    })
}

fn error_response(id: Value, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32603,
            "message": message
        }
    })
}

fn handle_stable_tool(name: &str, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    match name {
        "athena_get_latest_state" => {
            let storage = DoltStorage::open(required_string(arguments, "db_path")?)?;
            let Some(purpose) = storage.latest_purpose()? else {
                return Ok(text_result(json!({
                    "purpose": null,
                    "packet": null
                })));
            };
            let packet = storage.latest_packet_for_purpose(&purpose.purpose_id)?;
            Ok(text_result(json!({
                "purpose": purpose,
                "packet": packet
            })))
        }
        "athena_create_purpose" => {
            let storage = DoltStorage::open(required_string(arguments, "db_path")?)?;
            let result = create_purpose(
                &storage,
                &required_string(arguments, "statement")?,
                &required_string(arguments, "success_criteria")?,
            )?;
            Ok(text_result(serde_json::to_value(result)?))
        }
        "athena_update_purpose" => {
            let storage = DoltStorage::open(required_string(arguments, "db_path")?)?;
            let result = update_purpose(
                &storage,
                &PurposeId::new(required_string(arguments, "purpose_id")?),
                &required_string(arguments, "statement")?,
                &required_string(arguments, "success_criteria")?,
            )?;
            Ok(text_result(serde_json::to_value(result)?))
        }
        "athena_apply_feedback" => {
            let storage = DoltStorage::open(required_string(arguments, "db_path")?)?;
            let input = FeedbackApplyInput {
                fragment_feedback: serde_json::from_value(
                    arguments
                        .get("fragment_feedback")
                        .cloned()
                        .ok_or_else(|| io::Error::other("missing fragment_feedback"))?,
                )?,
                new_fragments: serde_json::from_value(
                    arguments
                        .get("new_fragments")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                )?,
            };
            let result = apply_feedback_command(
                &storage,
                &PurposeId::new(required_string(arguments, "purpose_id")?),
                &PacketId::new(required_string(arguments, "packet_id")?),
                parse_outcome(&required_string(arguments, "outcome")?)?,
                input,
            )?;
            Ok(text_result(serde_json::to_value(result)?))
        }
        other => Err(io::Error::other(format!("unknown stable tool: {other}")).into()),
    }
}

fn handle_dev_tool(name: &str, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let fixture_path = PathBuf::from(required_string(arguments, "fixture_path")?);
    let request = match name {
        "athena_dev_assemble_packet" => AthenaRequest::AssemblePacket {
            prompt: required_string(arguments, "prompt")?,
            success_criteria: required_string(arguments, "success_criteria")?,
        },
        "athena_dev_check_orientation" => AthenaRequest::CheckOrientation {
            purpose: serde_json::from_value(
                arguments
                    .get("purpose")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing purpose"))?,
            )?,
            packet: serde_json::from_value(
                arguments
                    .get("packet")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing packet"))?,
            )?,
            response: serde_json::from_value(
                arguments
                    .get("response")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing response"))?,
            )?,
        },
        "athena_dev_apply_feedback" => AthenaRequest::ApplyFeedback {
            purpose: serde_json::from_value(
                arguments
                    .get("purpose")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing purpose"))?,
            )?,
            packet: serde_json::from_value(
                arguments
                    .get("packet")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing packet"))?,
            )?,
            feedback: serde_json::from_value(
                arguments
                    .get("feedback")
                    .cloned()
                    .ok_or_else(|| io::Error::other("missing feedback"))?,
            )?,
        },
        other => return Err(io::Error::other(format!("unknown dev tool: {other}")).into()),
    };

    let response = handle_request(request, fixture_path)?;
    Ok(text_result(serde_json::to_value(response)?))
}

fn dispatch(
    mode: Mode,
    method: &str,
    params: Value,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": if mode == Mode::Stable { "athena" } else { "athena-dev" },
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": if mode == Mode::Stable {
                "Stable Athena MCP server. Use persisted tools by default."
            } else {
                "Dev Athena MCP server. Stateless experimentation only."
            }
        }))),
        "notifications/initialized" => Ok(None),
        "tools/list" => Ok(Some(json!({
            "tools": if mode == Mode::Stable { stable_tools() } else { dev_tools() }
        }))),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::other("missing tool name"))?;
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let result = if mode == Mode::Stable {
                handle_stable_tool(name, &arguments)?
            } else {
                handle_dev_tool(name, &arguments)?
            };
            Ok(Some(result))
        }
        "ping" => Ok(Some(json!({}))),
        other if other.starts_with("notifications/") => Ok(None),
        other => Err(io::Error::other(format!("unsupported method: {other}")).into()),
    }
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }

    let mut body =
        vec![0; content_length.ok_or_else(|| io::Error::other("missing Content-Length"))?];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn write_message(writer: &mut impl Write, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_mode()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = io::BufWriter::new(stdout.lock());

    while let Some(message) = read_message(&mut reader)? {
        let id = message.get("id").cloned();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| io::Error::other("missing method"))?;
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

        match dispatch(mode, method, params) {
            Ok(Some(result)) => {
                if let Some(id) = id {
                    write_message(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": result
                        }),
                    )?;
                }
            }
            Ok(None) => {}
            Err(err) => {
                if let Some(id) = id {
                    write_message(&mut writer, &error_response(id, err.to_string()))?;
                }
            }
        }
    }

    Ok(())
}
