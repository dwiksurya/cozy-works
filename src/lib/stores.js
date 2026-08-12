import { writable, derived, get } from "svelte/store";

// ---- Settings (persisted JSON + SQLite) ----
export const settings = writable({
  lang: "en",
  theme: "zen-dark",
  // pomodoro
  focusMinutes: 25,
  shortBreak: 5,
  longBreak: 15,
  longEvery: 4,
  autoStartBreak: false,
  autoStartFocus: false,
  soundOn: true,
  // music
  musicDir: "",
  // terminal
  terminalShell: "",
});

export const settingsLoaded = writable(false);

// ---- UI ----
export const activeView = writable("dashboard");
export const toastMsg = writable(null); // {type, text, id}

export const sidebarCollapsed = writable(false);

// ---- Workspace status (git branch of active terminal) ----
export const workspaceStatus = writable({
  branch: null,
  dirty: false,
  dir: "",
});

export const terminalTabs = writable([]); // [{id, label, createdAt}] — managed by Terminal component

// ---- Agents (auto-detected from terminal processes) ----
export const agents = writable([]); // [{terminal_id, name, status, pid}]

export function toast(text, type = "info") {
  toastMsg.set({ text, type, id: Date.now() });
  setTimeout(() => toastMsg.set(null), 3500);
}

// ---- Pomodoro ----
export const pomodoro = writable({
  running: false,
  phase: "focus", // focus | short | long
  remaining: 25 * 60,
  total: 25 * 60,
  completedToday: 0,
  todayMinutes: 0,
});

// ---- Todo ----
export const todos = writable([]); // [{id,title,notes,priority,tag,due_date,repeat,status,done_at,position,items:[{id,label,done}]}]

// ---- Notes / Memos ----
export const notes = writable([]);
export const memos = writable([]);

// ---- Music ----
export const musicState = writable({
  library: [],
  queue: [],
  current: null,
  playing: false,
  position: 0,
  duration: 0,
  shuffle: false,
  repeat: false,
  volume: 0.7,
});
