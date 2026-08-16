use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

pub struct PtyState {
    pub sessions: Mutex<HashMap<u32, PtySession>>,
    pub next_id: Mutex<u32>,
}

pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    writer: Option<Box<dyn std::io::Write + Send>>,
    /// Master PTY handle — kept for resize() (takes &self)
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// Last ~4KB of terminal output (for agent screen-based detection)
    last_output: Arc<Mutex<String>>,
    /// Shell child PID (for agent process detection)
    shell_pid: Option<u32>,
    /// Last OSC 0/2 title emitted by the PTY (agents publish status via title)
    osc_title: Arc<Mutex<Option<String>>>,
    /// Last published agent status: "running" | "blocker" | "idle"
    status: Mutex<String>,
    /// When a working→idle transition was first observed (anti-flicker)
    pending_idle: Mutex<Option<Instant>>,
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
        let shell_name = shell.rsplit('/').next().unwrap_or("bash");
        let mut cb = CommandBuilder::new(&shell);
        // login + interactive so user profile (~/.bashrc, ~/.zshrc, nvm, cargo)
        // is loaded — otherwise PATH is incomplete and agents (claude, codex) not found
        match shell_name {
            "bash" => {
                cb.arg("-il");
            }
            "zsh" => {
                cb.arg("-il");
            }
            "fish" => {
                cb.arg("-il");
            }
            _ => {
                cb.arg("-il");
            }
        }
        cb
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

    let master: Box<dyn MasterPty + Send> = pair.master.into();
    let master = Arc::new(Mutex::new(master));
    let mut reader = master
        .lock()
        .unwrap()
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let writer = master
        .lock()
        .unwrap()
        .take_writer()
        .map_err(|e| e.to_string())?;

    let mut next = state.next_id.lock().unwrap();
    let id = *next;
    *next += 1;
    drop(next);

    let last_output = Arc::new(Mutex::new(String::new()));
    let osc_title = Arc::new(Mutex::new(None::<String>));

    state.sessions.lock().unwrap().insert(
        id,
        PtySession {
            child,
            writer: Some(writer),
            master: master.clone(),
            last_output: last_output.clone(),
            shell_pid,
            osc_title: osc_title.clone(),
            status: Mutex::new("running".to_string()),
            pending_idle: Mutex::new(None),
        },
    );

    let handle = app.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 4096];
        // OSC title tracker: 0=ground, 1=esc, 2=in-osc, 3=esc-in-osc
        let mut osc_state = 0u8;
        let mut osc_buf: Vec<u8> = Vec::with_capacity(256);
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // parse OSC 0/2 title sequences from the byte stream
                    for &byte in &buf[..n] {
                        match osc_state {
                            0 => {
                                if byte == 0x1b {
                                    osc_state = 1;
                                }
                            }
                            1 => {
                                if byte == b']' {
                                    osc_buf.clear();
                                    osc_state = 2;
                                } else if byte == 0x1b {
                                    // stay in esc
                                } else {
                                    osc_state = 0;
                                }
                            }
                            2 => match byte {
                                0x07 => {
                                    finalize_osc(&osc_title, &osc_buf);
                                    osc_state = 0;
                                }
                                0x1b => osc_state = 3,
                                _ => {
                                    if osc_buf.len() < 1024 {
                                        osc_buf.push(byte);
                                    }
                                }
                            },
                            _ => {
                                if byte == b'\\' {
                                    finalize_osc(&osc_title, &osc_buf);
                                    osc_state = 0;
                                } else if byte != 0x1b {
                                    osc_buf.push(byte);
                                    osc_state = 2;
                                }
                            }
                        }
                    }
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    // keep ring buffer of last output
                    let mut out = last_output.lock().unwrap();
                    out.push_str(&data);
                    if out.len() > 4096 {
                        *out = out
                            .chars()
                            .rev()
                            .take(4096)
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect();
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

/// Parse an OSC body `(<num>;...<payload>)`; keep it when it's a title
/// sequence (OSC 0 / OSC 2) so agents can publish status via their title.
fn finalize_osc(osc_title: &Arc<Mutex<Option<String>>>, body: &[u8]) {
    let text = String::from_utf8_lossy(body);
    let Some((num, payload)) = text.split_once(';') else {
        return;
    };
    if num == "0" || num == "2" {
        let mut title = osc_title.lock().unwrap();
        *title = Some(payload.trim().to_string());
    }
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

/// Resize a PTY to the given rows/cols (used by multiplexer pane resize).
#[tauri::command]
pub fn resize_terminal(state: State<PtyState>, id: u32, rows: u16, cols: u16) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&id)
        .ok_or_else(|| format!("terminal {id} not found"))?;
    let master = session.master.lock().unwrap();
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())
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

// ===========================================================================
// Agent detection
//
// Two signals, strongest first:
//   1. OSC 0/2 title — agents publish live status via terminal title
//      (Claude: braille spinner while working; hermes: ⏳/⚠/✓; codex: spinner)
//   2. Screen tail — fallback patterns matched against the last 4KB of output
// Plus a pending-idle confirmation to avoid flicker on working→idle.
// ===========================================================================

#[derive(Serialize)]
pub struct AgentInfo {
    terminal_id: u32,
    name: String,
    status: String,
    pid: u32,
    title: Option<String>,
}

/// Per-pane info for the multiplexer UI: process name (shell or agent) +
/// agent status + OSC title.
#[derive(Serialize)]
pub struct PaneInfo {
    terminal_id: u32,
    /// Process name: agent name if an agent runs in this pane, else shell name
    name: String,
    /// "running" | "blocker" | "idle" (agent status; "running" for plain shell)
    status: String,
    pid: u32,
    title: Option<String>,
}

/// Rules per agent — ported from herdr's bundled manifests (detect/manifests).
/// Priority order: blocker > running > idle; OSC beats screen fallback.
struct AgentRules {
    osc_blocked: &'static [&'static str],
    osc_working: &'static [&'static str],
    /// Spinner/status chars that mark "working" (braille, half-circles, ⏳…)
    osc_working_chars: &'static [char],
    osc_idle: &'static [&'static str],
    osc_idle_chars: &'static [char],
    /// Status chars in the OSC title that mark "blocked" (⚠, ⌛…)
    osc_blocked_chars: &'static [char],
    screen_blocked: &'static [&'static str],
    screen_working: &'static [&'static str],
    screen_idle: &'static [&'static str],
}

const SPINNER_CHARS: &[char] = &[
    '⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏', '⠁', '⠂', '⠄', '⡀', '⢀', '⣀',
];

fn agent_rules(agent: &str) -> Option<AgentRules> {
    match agent {
        "claude" => Some(AgentRules {
            osc_blocked: &["esc to cancel", "enter to confirm"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &['✳', '✳'],
            osc_blocked_chars: &['⚠'],
            screen_blocked: &[
                "do you want to proceed?",
                "would you like to",
                "waiting for permission",
                "do you want to allow",
                "tab to amend",
                "ctrl+e to explain",
                "review your answers",
                "skip interview and plan immediately",
            ],
            screen_working: &["/btw", "esc to interrupt"],
            screen_idle: &[],
        }),
        "codex" => Some(AgentRules {
            osc_blocked: &["Action Required"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &[
                "press enter to confirm or esc to cancel",
                "enter to submit answer",
                "enter to submit all",
                "allow command?",
                "do you trust the contents of this directory?",
                "[y/n]",
                "yes (y)",
            ],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "opencode" => Some(AgentRules {
            osc_blocked: &["Permission required"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["△ Permission required", "esc dismiss"],
            screen_working: &["esc to interrupt", "ctrl+c to interrupt", "press esc to interrupt"],
            screen_idle: &[],
        }),
        "hermes" => Some(AgentRules {
            osc_blocked: &[],
            osc_working: &[],
            osc_working_chars: &['⏳'],
            osc_idle: &[],
            osc_idle_chars: &['✓'],
            osc_blocked_chars: &['⚠'],
            screen_blocked: &[
                "hermes needs your",
                "dangerous",
                "approval",
                "allow once",
                "type your answer",
                "sudo password",
                "enter confirm",
                "approve once",
                "start a new session",
            ],
            screen_working: &["msg=interrupt", "ctrl+c to interrupt", "ctrl+c cancel"],
            screen_idle: &[],
        }),
        "pi" => Some(AgentRules {
            osc_blocked: &[],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["waiting for input", "esc to cancel"],
            screen_working: &["Working..."],
            screen_idle: &[],
        }),
        "gemini" => Some(AgentRules {
            osc_blocked: &[],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["allow command?", "proceed?"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "cursor" => Some(AgentRules {
            osc_blocked: &["waiting for permission", "needs approval"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["waiting for your approval", "approve"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "copilot" => Some(AgentRules {
            osc_blocked: &["needs review", "permission required"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["Do you want to"],
            screen_working: &["Working"],
            screen_idle: &[],
        }),
        "qwen" => Some(AgentRules {
            osc_blocked: &["Action Required"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["press enter to confirm or esc to cancel", "[y/n]"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "kimi" => Some(AgentRules {
            osc_blocked: &["press enter to confirm or esc to cancel"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["[y/n]", "do you want to"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "kilo" => Some(AgentRules {
            osc_blocked: &["Action Required"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["press enter to confirm or esc to cancel"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "grok" => Some(AgentRules {
            osc_blocked: &["Action Required", "press enter to confirm"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["do you want to"],
            screen_working: &["esc to interrupt"],
            screen_idle: &[],
        }),
        "devin" => Some(AgentRules {
            osc_blocked: &["waiting for your input"],
            osc_working: &[],
            osc_working_chars: SPINNER_CHARS,
            osc_idle: &[],
            osc_idle_chars: &[],
            osc_blocked_chars: &[],
            screen_blocked: &["need your input", "confirm"],
            screen_working: &["Working"],
            screen_idle: &[],
        }),
        _ => None,
    }
}

fn has_any_char(text: &str, chars: &[char]) -> bool {
    text.chars().any(|c| chars.contains(&c))
}

/// Compute desired status from OSC title + screen tail.
/// Returns (status, visible_idle): visible_idle=true when the idle state was
/// explicitly signalled (spinner stopped / checkmark), so we don't debounce it.
fn compute_status(agent: &str, osc: Option<&str>, screen: &str) -> (String, bool) {
    let Some(rules) = agent_rules(agent) else {
        // unknown agent → keep the legacy heuristic
        let blocker = looks_like_blocker(screen);
        return (if blocker { "blocker".to_string() } else { "running".to_string() }, false);
    };

    if let Some(osc) = osc {
        let t = osc.trim();
        if !t.is_empty() {
            if rules.osc_blocked.iter().any(|p| t.contains(p))
                || has_any_char(t, rules.osc_blocked_chars)
            {
                return ("blocker".to_string(), false);
            }
            if rules.osc_working.iter().any(|p| t.contains(p))
                || has_any_char(t, rules.osc_working_chars)
            {
                return ("running".to_string(), false);
            }
            if rules.osc_idle.iter().any(|p| t.contains(p)) || has_any_char(t, rules.osc_idle_chars)
            {
                return ("idle".to_string(), true);
            }
        }
    }

    if rules
        .screen_blocked
        .iter()
        .any(|p| screen.to_lowercase().contains(&p.to_lowercase()))
    {
        return ("blocker".to_string(), false);
    }
    if rules
        .screen_working
        .iter()
        .any(|p| screen.to_lowercase().contains(&p.to_lowercase()))
    {
        return ("running".to_string(), false);
    }
    if rules
        .screen_idle
        .iter()
        .any(|p| screen.to_lowercase().contains(&p.to_lowercase()))
    {
        return ("idle".to_string(), true);
    }

    // OSC title present but not matched → agent is sitting at its prompt
    if osc.is_some_and(|o| !o.trim().is_empty()) {
        return ("idle".to_string(), false);
    }

    ("running".to_string(), false)
}

/// Anti-flicker: a working→idle transition is only published after it holds
/// for `PENDING_IDLE_HOLD` (ported from herdr's PendingIdleConfirmation).
const PENDING_IDLE_HOLD: Duration = Duration::from_secs(5);

fn update_status(
    cur: &mut String,
    pending: &mut Option<Instant>,
    now: Instant,
    agent: &str,
    osc: Option<&str>,
    screen: &str,
) -> String {
    let (new_status, visible_idle) = compute_status(agent, osc, screen);

    if cur.as_str() == "running" && new_status == "idle" && !visible_idle {
        match pending {
            Some(started) if now.duration_since(*started) >= PENDING_IDLE_HOLD => {
                *pending = None;
                *cur = "idle".to_string();
            }
            Some(_) => return "running".to_string(), // still holding
            None => {
                *pending = Some(now);
                return "running".to_string();
            }
        }
    } else {
        *pending = None;
        *cur = new_status.clone();
    }
    new_status
}

#[tauri::command]
pub fn list_agents(state: State<PtyState>) -> Vec<AgentInfo> {
    // Snapshot shell pids + per-session signals quickly, WITHOUT holding the
    // sessions lock during the (potentially slow) process scan. Holding it
    // across /proc recursion blocks write_terminal (user keystrokes) — input
    // would stall every 3s poll.
    let snapshots: Vec<(u32, Option<u32>, Option<String>, String, String, Option<Instant>)> = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .iter()
            .map(|(id, session)| {
                let osc = session.osc_title.lock().unwrap().clone();
                let last = session.last_output.lock().unwrap().clone();
                let status = session.status.lock().unwrap().clone();
                let pending = *session.pending_idle.lock().unwrap();
                (*id, session.shell_pid, osc, last, status, pending)
            })
            .collect()
    };

    let mut agents = Vec::new();
    for (id, shell_pid, osc, last, mut cur, mut pending) in snapshots {
        let Some(pid) = shell_pid else {
            continue;
        };
        let Some((agent_pid, name)) = find_agent_process(pid) else {
            continue;
        };
        let status = update_status(
            &mut cur,
            &mut pending,
            Instant::now(),
            &name,
            osc.as_deref(),
            &last,
        );
        // persist updated status back (best-effort; skip if session closed)
        if let Some(session) = state.sessions.lock().unwrap().get(&id) {
            *session.status.lock().unwrap() = cur;
            *session.pending_idle.lock().unwrap() = pending;
        }
        agents.push(AgentInfo {
            terminal_id: id,
            name,
            status,
            pid: agent_pid,
            title: osc,
        });
    }
    agents
}

/// Info for every PTY session: process name (agent if detected, else shell
/// basename), status, pid, OSC title. Used by pane titlebars in the mux UI.
#[tauri::command]
pub fn list_panes(state: State<PtyState>) -> Vec<PaneInfo> {
    // Snapshot quickly (same lock discipline as list_agents — never hold the
    // sessions lock during /proc scans, it stalls write_terminal).
    let snapshots: Vec<(u32, Option<u32>, Option<String>, String, String, Option<Instant>)> = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .iter()
            .map(|(id, session)| {
                let osc = session.osc_title.lock().unwrap().clone();
                let last = session.last_output.lock().unwrap().clone();
                let status = session.status.lock().unwrap().clone();
                let pending = *session.pending_idle.lock().unwrap();
                (*id, session.shell_pid, osc, last, status, pending)
            })
            .collect()
    };

    let mut panes = Vec::new();
    for (id, shell_pid, osc, last, mut cur, mut pending) in snapshots {
        let Some(pid) = shell_pid else {
            continue;
        };
        // default name: shell process basename
        let (name, agent_pid) = match find_agent_process(pid) {
            Some((ap, agent)) => (agent, ap),
            None => (process_name(pid).unwrap_or_else(|| "shell".to_string()), pid),
        };
        let status = update_status(
            &mut cur,
            &mut pending,
            Instant::now(),
            &name,
            osc.as_deref(),
            &last,
        );
        if let Some(session) = state.sessions.lock().unwrap().get(&id) {
            *session.status.lock().unwrap() = cur;
            *session.pending_idle.lock().unwrap() = pending;
        }
        panes.push(PaneInfo {
            terminal_id: id,
            name,
            status,
            pid: agent_pid,
            title: osc,
        });
    }
    panes
}

/// Best-effort recursive scan: direct children of the shell first, then one
/// level deeper (covers `bash -c "claude …"` wrappers).
fn find_agent_process(shell_pid: u32) -> Option<(u32, String)> {
    let children = get_child_processes(shell_pid);
    // 1. direct agent child (claude, codex, …)
    for (pid, name, _) in &children {
        if let Some(agent) = identify_agent(name) {
            return Some((*pid, agent.to_string()));
        }
    }
    // 2. wrapped agent (node -e "…claude…", bash -c "claude …")
    for (pid, _, argv) in &children {
        if let Some(agent) = wrapped_agent_from_argv(argv) {
            return Some((*pid, agent.to_string()));
        }
    }
    // 3. one level deeper (e.g. claude spawns a child process)
    for (pid, _, _) in &children {
        for (gpid, gname, gargv) in get_child_processes(*pid) {
            if let Some(agent) = identify_agent(&gname) {
                return Some((gpid, agent.to_string()));
            }
            if let Some(agent) = wrapped_agent_from_argv(&gargv) {
                return Some((gpid, agent.to_string()));
            }
        }
    }
    None
}

fn identify_agent(name: &str) -> Option<String> {
    let n = normalize_agent_name(name);
    let agent = match n.as_str() {
        "pi" | "pi-coding-agent" => "pi",
        "claude" | "claude-code" => "claude",
        "codex" => "codex",
        "gemini" => "gemini",
        "cursor" | "cursor-agent" => "cursor",
        "devin" | "devin-cli" => "devin",
        "agy" | "antigravity" | "antigravity-cli" => "agy",
        "cline" => "cline",
        "omp" => "omp",
        "mastracode" | "mastra-code" | "mastra code" => "mastracode",
        "opencode" | "opencode2" | "open-code" => "opencode",
        "copilot" | "github-copilot" | "ghcs" => "copilot",
        "kimi" | "kimi-code" | "kimi code" => "kimi",
        "kiro" | "kiro-cli" => "kiro",
        "droid" => "droid",
        "amp" | "amp-local" => "amp",
        "grok" | "grok-build" => "grok",
        "hermes" | "hermes-agent" => "hermes",
        "kilo" | "kilo-code" | "kilo code" => "kilo",
        "qodercli" | "qoder" | "qodercn" => "qodercli",
        "qwen" | "qwen-code" | "qwen code" => "qwen",
        "maki" => "maki",
        "aider" => "aider",
        _ => return None,
    };
    Some(agent.to_string())
}

fn normalize_agent_name(name: &str) -> String {
    let mut n = name.trim().to_lowercase();
    for suffix in [".exe", ".cmd", ".bat", ".ps1", ".js"] {
        if n.ends_with(suffix) {
            n.truncate(n.len() - suffix.len());
            break;
        }
    }
    n
}

/// Detect an agent hidden behind a runtime wrapper (`node -e "…"`, `bash -c "claude …"`).
fn wrapped_agent_from_argv(argv: &[String]) -> Option<String> {
    for token in argv.iter().skip(1) {
        if let Some(agent) = identify_agent(token) {
            return Some(agent);
        }
        // a command string like "claude --dangerously-skip-permissions"
        if let Some(first) = token.split_whitespace().next() {
            if let Some(agent) = identify_agent(first) {
                return Some(agent);
            }
        }
    }
    None
}

// Linux: read /proc/<pid>/task/<pid>/children for child pids, then cmdline
#[cfg(target_os = "linux")]
fn get_child_processes(shell_pid: u32) -> Vec<(u32, String, Vec<String>)> {
    let mut out = Vec::new();
    let task_dir = format!("/proc/{shell_pid}/task");
    if let Ok(entries) = std::fs::read_dir(&task_dir) {
        for entry in entries.flatten() {
            let children_file = entry.path().join("children");
            if let Ok(children_str) = std::fs::read_to_string(&children_file) {
                for pid_str in children_str.split_whitespace() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        let (name, argv) = process_info(pid);
                        if !name.is_empty() {
                            out.push((pid, name, argv));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn process_info(pid: u32) -> (String, Vec<String>) {
    if let Ok(cmd) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) {
        let parts: Vec<String> = cmd
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if let Some(first) = parts.first() {
            let base = first
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_lowercase();
            return (base, parts);
        }
    }
    (String::new(), Vec::new())
}

// Non-linux: best-effort via `ps`
#[cfg(not(target_os = "linux"))]
fn get_child_processes(_shell_pid: u32) -> Vec<(u32, String, Vec<String>)> {
    Vec::new()
}

/// Process basename for a pid (shell name fallback for pane titlebars).
fn process_name(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let (name, _) = process_info(pid);
        if !name.is_empty() {
            return Some(name);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let out = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

// Heuristic fallback for unknown agents: last output ends with an interactive
// prompt (agent waiting for input).
fn looks_like_blocker(last_output: &str) -> bool {
    let trimmed = last_output.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let markers = [
        "❯", "❯❯", "Proceed?", "Do you want", "(y/n)", "Y/n", "──",
    ];
    markers.iter().any(|m| trimmed.ends_with(m) || trimmed.contains(m))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc_parse_bel_terminated_title() {
        let title = Arc::new(Mutex::new(None));
        // "0;⠋ building app" — braille spinner U+280B is non-ASCII, use hex
        finalize_osc(&title, b"0;\xE2\xA0\x8B building app");
        assert_eq!(
            *title.lock().unwrap(),
            Some("⠋ building app".to_string())
        );
    }

    #[test]
    fn osc_parse_ignores_non_title() {
        let title = Arc::new(Mutex::new(None));
        finalize_osc(&title, b"9;4;foo");
        assert_eq!(*title.lock().unwrap(), None);
    }

    #[test]
    fn identify_agent_alias_mapping() {
        assert_eq!(identify_agent("claude").as_deref(), Some("claude"));
        assert_eq!(identify_agent("claude-code").as_deref(), Some("claude"));
        assert_eq!(identify_agent("codex.exe").as_deref(), Some("codex"));
        assert_eq!(identify_agent("opencode2").as_deref(), Some("opencode"));
        assert_eq!(identify_agent("hermes-agent").as_deref(), Some("hermes"));
        assert_eq!(identify_agent("github-copilot").as_deref(), Some("copilot"));
        assert_eq!(identify_agent("bash"), None);
        assert_eq!(identify_agent("ls"), None);
    }

    #[test]
    fn wrapped_agent_detected_from_argv() {
        let argv = vec!["bash".to_string(), "-c".to_string(), "claude --dangerously-skip-permissions".to_string()];
        assert_eq!(wrapped_agent_from_argv(&argv).as_deref(), Some("claude"));
        let argv2 = vec!["node".to_string(), "-e".to_string(), "opencode".to_string()];
        assert_eq!(wrapped_agent_from_argv(&argv2).as_deref(), Some("opencode"));
        let argv3 = vec!["git".to_string(), "status".to_string()];
        assert_eq!(wrapped_agent_from_argv(&argv3), None);
    }

    #[test]
    fn hermes_rules_have_blocked_char() {
        let rules = agent_rules("hermes").expect("hermes rules");
        assert!(rules.osc_blocked_chars.contains(&'⚠'), "hermes must flag ⚠ OSC title as blocked");
        let (s, _) = compute_status("hermes", Some("⚠ needs input"), "");
        assert_eq!(s, "blocker");
    }

    #[test]
    fn compute_status_osc_driven() {
        // claude working spinner via OSC
        let (s, _) = compute_status("claude", Some("⠋ thinking"), "");
        assert_eq!(s, "running");
        // hermes blocked
        let (s, _) = compute_status("hermes", Some("⚠ needs input"), "");
        assert_eq!(s, "blocker");
        // hermes idle checkmark
        let (s, visible) = compute_status("hermes", Some("✓ done"), "");
        assert_eq!(s, "idle");
        assert!(visible);
        // codex blocked via screen fallback
        let (s, _) = compute_status(
            "codex",
            Some("codex"),
            "Do you trust the contents of this directory?",
        );
        assert_eq!(s, "blocker");
    }

    #[test]
    fn update_status_debounces_working_to_idle() {
        let now = Instant::now();
        let mut cur = "running".to_string();
        let mut pending = None;
        // OSC title present but not matched (e.g. "claude" at prompt) → idle,
        // but working→idle must be debounced
        let s1 = update_status(&mut cur, &mut pending, now, "claude", Some("claude"), "");
        assert_eq!(s1, "running");
        assert!(pending.is_some());
        // before hold elapses: still running
        let s2 = update_status(
            &mut cur,
            &mut pending,
            now + Duration::from_secs(2),
            "claude",
            Some("claude"),
            "",
        );
        assert_eq!(s2, "running");
        // after 5s: idle
        let s3 = update_status(
            &mut cur,
            &mut pending,
            now + Duration::from_secs(6),
            "claude",
            Some("claude"),
            "",
        );
        assert_eq!(s3, "idle");
    }

    #[test]
    fn update_status_immediate_when_visible_idle() {
        let now = Instant::now();
        let mut cur = "running".to_string();
        let mut pending = None;
        // hermes ✓ is an explicit idle signal → no debounce
        let s = update_status(&mut cur, &mut pending, now, "hermes", Some("✓ done"), "");
        assert_eq!(s, "idle");
        assert!(pending.is_none());
    }
}
