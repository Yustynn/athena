use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".athena/fragments.json")
}

fn unique_repo_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("athena-mcp-{nanos}"))
}

struct McpSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpSession {
    fn new(mode: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_athena-mcp"))
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut session = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        session.initialize();
        session
    }

    fn initialize(&mut self) {
        let result = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "athena-test",
                    "version": "0.1.0"
                }
            }),
        );
        assert_eq!(result["protocolVersion"], "2025-11-25");

        self.notify("notifications/initialized", json!({}));
    }

    fn notify(&mut self, method: &str, params: Value) {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send(&message);
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send(&message);

        let response = self.read_message();
        assert_eq!(response["id"], id);
        if response.get("error").is_some() {
            panic!("mcp error response: {response}");
        }
        response["result"].clone()
    }

    fn send(&mut self, value: &Value) {
        let body = serde_json::to_vec(value).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.stdin.write_all(&body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_message(&mut self) -> Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).unwrap();
            if read == 0 {
                let status = self.child.wait().unwrap();
                panic!("mcp child exited before response: {status}");
            }
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                content_length = Some(value.trim().parse::<usize>().unwrap());
            }
        }

        let mut body = vec![0; content_length.expect("content length header missing")];
        self.stdout.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn stable_mcp_exposes_persisted_athena_tools() {
    let repo_path = unique_repo_path();
    let mut session = McpSession::new("stable");

    let tools = session.request("tools/list", json!({}));
    let tool_names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"athena_create_purpose"));
    assert!(tool_names.contains(&"athena_get_latest_state"));
    assert!(!tool_names.contains(&"athena_dev_assemble_packet"));

    let create = session.request(
        "tools/call",
        json!({
            "name": "athena_create_purpose",
            "arguments": {
                "db_path": repo_path,
                "fixture_path": fixture_path(),
                "statement": "Use athena during codex work",
                "success_criteria": "packet is persisted"
            }
        }),
    );
    let purpose = &create["structuredContent"]["purpose"];
    let packet = &create["structuredContent"]["packet"];
    assert_eq!(purpose["statement"], "Use athena during codex work");
    assert_eq!(packet["purpose_id"], purpose["purpose_id"]);

    let latest = session.request(
        "tools/call",
        json!({
            "name": "athena_get_latest_state",
            "arguments": {
                "db_path": repo_path
            }
        }),
    );
    assert_eq!(
        latest["structuredContent"]["purpose"]["purpose_id"],
        purpose["purpose_id"]
    );
    assert_eq!(
        latest["structuredContent"]["packet"]["packet_id"],
        packet["packet_id"]
    );
}

#[test]
fn dev_mcp_exposes_stateless_athena_tools() {
    let mut session = McpSession::new("dev");

    let tools = session.request("tools/list", json!({}));
    let tool_names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"athena_dev_assemble_packet"));
    assert!(!tool_names.contains(&"athena_create_purpose"));

    let assemble = session.request(
        "tools/call",
        json!({
            "name": "athena_dev_assemble_packet",
            "arguments": {
                "fixture_path": fixture_path(),
                "prompt": "Use athena during codex work",
                "success_criteria": "packet helps next action"
            }
        }),
    );
    let purpose = &assemble["structuredContent"]["purpose"];
    let packet = &assemble["structuredContent"]["packet"];
    assert_eq!(purpose["status"], "active");
    assert_eq!(packet["purpose_id"], purpose["purpose_id"]);
    assert_eq!(packet["fragments"].as_array().unwrap().len(), 3);
}
