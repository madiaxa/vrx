use std::{
    io::Write,
    process::{Command, Stdio},
};
use vrx::Result;

#[test]
fn agent_protocol_keeps_reading_and_recovers_from_bad_requests() -> Result<()> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vrx"))
        .arg("api")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let input = r#"{"command":"text","text":"one two three four five"}
{"command":"adaptive","enabled":false}
{"command":"play"}
{"command":"advance","nanoseconds":200000000}
{"command":"jump","words":2}
{"command":"speed","wpm":600}
{"command":"unknown"}
{"command":"status"}
{"command":"shutdown"}
"#;
    child
        .stdin
        .take()
        .ok_or("No stdin pipe")?
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let lines: Vec<serde_json::Value> = output
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let status = lines.get(8).ok_or("Missing status")?;
    assert_eq!(
        status.get("state").and_then(|v| v.as_str()),
        Some("playing")
    );
    assert_eq!(status.get("index").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(status.get("wpm").and_then(|v| v.as_u64()), Some(600));
    assert_eq!(
        lines
            .get(7)
            .and_then(|v| v.get("ok"))
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    Ok(())
}
