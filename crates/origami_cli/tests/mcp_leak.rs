//! The MCP server is long-lived: its memory must not grow with the number of
//! requests. Measures the resident set size of `origami --mcp` over a few
//! hundred `format` calls (Linux: /proc, macOS: `ps`; skipped elsewhere).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};

/// Warm-up calls before the first measurement (allocator pools, caches).
const WARM_UP: u32 = 100;
/// Calls between the two measurements.
const CALLS: u32 = 800;
/// Allowed growth over `CALLS` calls. An audit measured 16 MB -> 20 MB over
/// 800 calls including start-up; keeping one formatted source per call
/// would add more than 10 MB here.
const MAX_GROWTH_KB: u64 = 6 * 1024;

/// Resident set size of `pid` in KiB.
fn rss_kb(pid: u32) -> Option<u64> {
    if cfg!(target_os = "linux") {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
        line.split_whitespace().nth(1)?.parse().ok()
    } else if cfg!(target_os = "macos") {
        let out = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        String::from_utf8(out.stdout).ok()?.trim().parse().ok()
    } else {
        None
    }
}

struct Server {
    child: Child,
    out: BufReader<ChildStdout>,
}

impl Server {
    fn start(dir: &std::path::Path) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_origami"))
            .arg("--mcp")
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Server { child, out }
    }

    fn call(&mut self, request: &str) -> serde_json::Value {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        self.out.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_server_memory_stays_flat_over_many_requests() {
    if rss_kb(std::process::id()).is_none() {
        eprintln!("skipped: no RSS measurement on this platform");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut server = Server::start(dir.path());
    let init = server.call(
        r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}"#,
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "origami");
    // About 10 KB of ABAP that needs formatting.
    let source: String = (0..200)
        .map(|i| format!("DATA lv_{i}   TYPE   i.\nlv_{i}   =   {i}  +  1.\n"))
        .collect();
    let request = |id: u32| {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": "format", "arguments": {"source": source, "filename": "z.prog.abap"}}
        })
        .to_string()
    };
    for id in 1..=WARM_UP {
        let r = server.call(&request(id));
        assert_eq!(r["result"]["structuredContent"]["changed"], true, "{r}");
    }
    let before = rss_kb(server.child.id()).unwrap();
    for id in WARM_UP + 1..=WARM_UP + CALLS {
        let r = server.call(&request(id));
        assert_eq!(r["id"], id);
    }
    let after = rss_kb(server.child.id()).unwrap();
    eprintln!("RSS after {WARM_UP} calls: {before} KiB, after {CALLS} more: {after} KiB");
    assert!(
        after.saturating_sub(before) <= MAX_GROWTH_KB,
        "RSS grew from {before} KiB to {after} KiB over {CALLS} calls"
    );
}
