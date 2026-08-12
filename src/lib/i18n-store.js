import { writable, derived, get } from "svelte/store";
import { messages } from "./i18n.js";
import { settings } from "./stores.js";

export const lang = writable("en");

export const t = derived([lang], ([$lang]) => {
  return messages[$lang] || messages.en;
});

export function fmtTime(seconds) {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function fmtDate(dateStr) {
  if (!dateStr) return "";
  const d = new Date(dateStr);
  return d.toLocaleDateString(get(lang) === "id" ? "id-ID" : "en-US", {
    month: "short",
    day: "numeric",
  });
}

export function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

export function nowIso() {
  return new Date().toISOString();
}
