use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

pub struct PtyState {
    pub sessions: Mutex<HashMap<u32, PtySession>>,
    pub next_id: Mutex<u32>,
}

pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    writer: Option<Box<dyn std::io::Write + Send>>,
    /// Last ~2KB of terminal output (for agent blocker detection)
    last_output: Arc<Mutex<String>>,
    /// Shell child PID (for agent process detection)
    shell_pid: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct PtyOutput {
    pub id: u32,
    pub data: String,
}

#[derive(Serialize, Clone)]
pub struct PtyExit {
    pub id: u32,
}

#[derive(Serialize)]
pub struct TermInfo {
    pub id: u32,
}

pub fn init(app: AppHandle) {
    app.manage(PtyState {
        sessions: Mutex::new(HashMap::new()),
        next_id: Mutex::new(1),
    });
}

fn shell_command() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("powershell.exe")
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        CommandBuilder::new(&shell)
    }
}

#[tauri::command]
pub fn spawn_terminal(app: AppHandle, state: State<PtyState>) -> Result<TermInfo, String> {
    let pty_system = native_pty_system();
    let cmd = shell_command();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let slave = pair.slave;
    let child = slave
        .spawn_command(cmd)
        .map_err(|e| format!("failed to spawn shell: {e}"))?;

    let shell_pid = child.process_id();

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let mut next = state.next_id.lock().unwrap();
    let id = *next;
    *next += 1;
    drop(next);

    let last_output = Arc::new(Mutex::new(String::new()));

    state.sessions.lock().unwrap().insert(
        id,
        PtySession {
            child,
            writer: Some(writer),
            last_output: last_output.clone(),
            shell_pid,
        },
    );

    let handle = app.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    // keep ring buffer of last output
                    let mut out = last_output.lock().unwrap();
                    out.push_str(&data);
                    if out.len() > 4096 {
                        *out = out.chars().rev().take(4096).collect::<String>().chars().rev().collect();
                    }
                    drop(out);
                    let _ = handle.emit("pty://output", PtyOutput { id, data });
                }
                Err(_) => break,
            }
        }
        let _ = handle.emit("pty://exit", PtyExit { id });
    });

    Ok(TermInfo { id })
}

#[tauri::command]
pub fn write_terminal(state: State<PtyState>, id: u32, data: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get_mut(&id)
        .ok_or_else(|| format!("terminal {id} not found"))?;
    let writer = session
        .writer
        .as_mut()
        .ok_or_else(|| "terminal writer unavailable".to_string())?;
    use std::io::Write;
    writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_terminal(state: State<PtyState>, id: u32) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(mut session) = sessions.remove(&id) {
        let _ = session.child.kill();
    }
    Ok(())
}

#[tauri::command]
pub fn list_terminals(state: State<PtyState>) -> Vec<u32> {
    state.sessions.lock().unwrap().keys().copied().collect()
}

#[derive(Serialize)]
pub struct GitStatus {
    pub dir: String,
    pub branch: Option<String>,
    pub dirty: bool,
}

// ---- Agent detection ----
const AGENT_CMDS: &[&str] = &[
    "claude", "codex", "opencode", "pi", "gemini", "cursor-agent", "aider", "copilot", "cline",
];

#[derive(Serialize, Clone)]
pub struct AgentInfo {
    pub terminal_id: u32,
    pub name: String,
    pub status: String, // "running" | "idle" | "blocker"
    pub pid: u32,
}

#[tauri::command]
pub fn list_agents(state: State<PtyState>) -> Vec<AgentInfo> {
    let sessions = state.sessions.lock().unwrap();
    let mut agents = Vec::new();
    for (id, session) in sessions.iter() {
        let shell_pid = match session.shell_pid {
            Some(p) => p,
            None => continue,
        };
        // find child processes of shell that match agent commands
        let children = get_child_processes(shell_pid);
        for (pid, name) in children {
            if AGENT_CMDS.iter().any(|a| name.contains(a)) {
                let last = session.last_output.lock().unwrap().clone();
                let status = if looks_like_blocker(&last) {
                    "blocker".to_string()
                } else {
                    "running".to_string()
                };
                agents.push(AgentInfo {
                    terminal_id: *id,
                    name,
                    status,
                    pid,
                });
            }
        }
    }
    agents
}

// Linux: read /proc/<pid>/task/<pid>/children for child pids, then cmdline
#[cfg(target_os = "linux")]
fn get_child_processes(shell_pid: u32) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let task_dir = format!("/proc/{shell_pid}/task");
    if let Ok(entries) = std::fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let children_file = entry.path().join("children");
            if let Ok(children_str) = std::fs::read_to_string(&children_file) {
                for pid_str in children_str.split_whitespace() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        let name = process_name(pid);
                        if !name.is_empty() {
                            out.push((pid, name));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn process_name(pid: u32) -> String {
    // command line
    if let Ok(cmd) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) {
        let parts: Vec<&str> = cmd.split('\0').filter(|s| !s.is_empty()).collect();
        if !parts.is_empty() {
            let base = parts[0]
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_lowercase();
            return base;
        }
    }
    String::new()
}

// Non-linux: best-effort via `ps`
#[cfg(not(target_os = "linux"))]
fn get_child_processes(_shell_pid: u32) -> Vec<(u32, String)> {
    Vec::new()
}

#[cfg(not(target_os = "linux"))]
fn process_name(_pid: u32) -> String {
    String::new()
}

// Heuristic: last output ends with an interactive prompt (agent waiting for input)
fn looks_like_blocker(last_output: &str) -> bool {
    let trimmed = last_output.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let markers = ["❯", "❯❯", "?", "> ", "$ ", "# ", "Proceed?", "Do you want", "(y/n)", "Y/n"];
    markers.iter().any(|m| trimmed.ends_with(m) || trimmed.contains("──"))
}

#[tauri::command]
pub fn git_branch(cwd: String) -> Result<GitStatus, String> {
    // Resolve ~ to home dir
    let resolved = if cwd == "~" || cwd.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| ".".into())
    } else if let Some(rest) = cwd.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/{rest}")
    } else {
        cwd.clone()
    };

    let path = PathBuf::from(&resolved);
    let dir_display = path.display().to_string();

    let output = std::process::Command::new("git")
        .args(["-C", &resolved, "rev-parse", "--abbrev-ref", "HEAD"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let dirty = std::process::Command::new("git")
                .args(["-C", &resolved, "status", "--porcelain"])
                .output()
                .map(|o| !o.stdout.is_empty())
                .unwrap_or(false);
            Ok(GitStatus {
                dir: dir_display,
                branch: Some(branch),
                dirty,
            })
        }
        _ => Ok(GitStatus {
            dir: dir_display,
            branch: None,
            dirty: false,
        }),
    }
}
