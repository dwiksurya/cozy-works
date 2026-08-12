use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct MusicFile {
    pub path: String,
    pub name: String,
}

#[tauri::command]
pub fn scan_music(dir: String) -> Vec<MusicFile> {
    let mut out = Vec::new();
    let exts = ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"];
    let root = Path::new(&dir);
    if !root.exists() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    let mut seen = std::collections::HashSet::new();
    while let Some(p) = stack.pop() {
        if !seen.insert(p.clone()) {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext.to_lowercase().as_str()) {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        out.push(MusicFile {
                            path: path.display().to_string(),
                            name,
                        });
                    }
                }
            }
        }
    }
    out
}
