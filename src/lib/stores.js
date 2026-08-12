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
  // ambient
  ambientMode: "auto", // auto | manual
  ambientScene: "morning", // when manual
  ambientVolume: 0.4,
  // ai
  aiModel: "ds/deepseek-v4-flash",
  aiBaseUrl: "https://router.takora.dev/v1",
  aiNotifyOnDone: true,
  aiNotifyOnAsk: true,
  // music
  musicDir: "",
  // pet
  pet: "cat",
  avatarHair: "#8ea8ff",
  avatarSkin: "#ffd9b8",
  // terminal
  terminalShell: "",
});

export const settingsLoaded = writable(false);

// ---- UI ----
export const activeView = writable("dashboard");
export const showAiSidebar = writable(false);
export const aiSidebarTab = writable("chat"); // chat | agent
export const toastMsg = writable(null); // {type, text, id}

export const sidebarCollapsed = writable(false);

// ---- Workspace status (git branch of active terminal, AI activity) ----
export const workspaceStatus = writable({
  branch: null,
  dirty: false,
  dir: "",
  aiRunning: false, // true while AI is streaming
  aiAction: "", // "thinking" | "needs-confirm" | "done"
});

export const terminalTabs = writable([]); // [{id, label, createdAt}] — managed by Terminal component

export function toast(text, type = "info") {
  toastMsg.set({ text, type, id: Date.now() });
  setTimeout(() => toastMsg.set(null), 3500);
}

// ---- Pet / Avatar state (sync with pomodoro & time) ----
export const petState = writable({
  animal: "cat",
  mood: "idle", // idle | focus | happy | sleepy | sad
  animation: "idle", // idle | blink | wave | sleep | jump
});

export const avatarState = writable({
  mood: "idle", // idle | focus | break | night
});

// ---- Pomodoro ----
export const pomodoro = writable({
  running: false,
  phase: "focus", // focus | short | long
  remaining: 25 * 60,
  total: 25 * 60,
  completedToday: 0,
  todayMinutes: 0,
});

// ---- Ambient ----
export const ambient = writable({
  mode: "auto",
  scene: "morning",
  playing: false,
  volume: 0.4,
});

export const timeOfDay = writable(getTimeOfDay());

function getTimeOfDay() {
  const h = new Date().getHours();
  if (h >= 5 && h < 11) return "morning";
  if (h >= 11 && h < 16) return "afternoon";
  if (h >= 16 && h < 19) return "evening";
  return "night";
}

// keep timeOfDay updated
setInterval(() => {
  const now = getTimeOfDay();
  timeOfDay.set(now);
}, 60_000);

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

// ---- derived: ambient effective scene ----
export const effectiveAmbientScene = derived([ambient, timeOfDay], ([$ambient, $tod]) => {
  if ($ambient.mode === "manual") return $ambient.scene;
  return $tod;
});

// ---- pet/avatar respond to time of day (sleep at night, calm at evening) ----
timeOfDay.subscribe((tod) => {
  if (tod === "night") {
    petState.update((p) => (p.mood === "focus" ? p : { ...p, mood: "sleepy", animation: "sleep" }));
    avatarState.update((a) => ({ ...a, mood: "night" }));
  } else if (tod === "evening") {
    petState.update((p) => (p.animation === "sleep" ? { ...p, mood: "idle", animation: "idle" } : p));
    avatarState.update((a) => ({ ...a, mood: "idle" }));
  } else {
    petState.update((p) => (p.animation === "sleep" ? { ...p, mood: "idle", animation: "idle" } : p));
    avatarState.update((a) => (a.mood === "night" ? { ...a, mood: "idle" } : a));
  }
});

// keep pet.animal synced with settings.pet
settings.subscribe((s) => {
  if (s.pet) petState.update((p) => ({ ...p, animal: s.pet }));
});
