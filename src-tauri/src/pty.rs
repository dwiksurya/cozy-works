use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct PtyState {
    pub sessions: Mutex<HashMap<u32, PtySession>>,
    pub next_id: Mutex<u32>,
}

pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    writer: Option<Box<dyn std::io::Write + Send>>,
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

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let mut next = state.next_id.lock().unwrap();
    let id = *next;
    *next += 1;
    drop(next);

    state.sessions.lock().unwrap().insert(
        id,
        PtySession {
            child,
            writer: Some(writer),
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

#[tauri::command]
pub fn git_branch(cwd: String) -> Result<GitStatus, String> {
    let path = PathBuf::from(&cwd);
    let dir_display = path.display().to_string();

    let output = std::process::Command::new("git")
        .args(["-C", &cwd, "rev-parse", "--abbrev-ref", "HEAD"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let dirty = std::process::Command::new("git")
                .args(["-C", &cwd, "status", "--porcelain"])
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
