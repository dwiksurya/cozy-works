<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Terminal } from "xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "xterm/css/xterm.css";
  import { t } from "../i18n-store.js";
  import { toast, workspaceStatus, terminalTabs } from "../stores.js";
  import { get } from "svelte/store";
  import Icon from "./Icon.svelte";

  // Tab model: { id (termId), label (path or name), term (xterm instance), fit, container }
  let tabs = [];
  let activeTabId = null;
  let containerRefs = {}; // id -> DOM element
  let unlisten = [];
  let statusTimer;
  let loading = true; // shell spawning indicator

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function homeDir() {
    return isTauri() ? (window.__TAURI_INTERNALS__?.invoke ? "" : "") : "";
  }

  function createTerm(opts = {}) {
    const term = new Terminal({
      fontSize: 13,
      fontFamily: "JetBrains Mono, ui-monospace, monospace",
      cursorBlink: true,
      scrollback: 5000,
      ...opts,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());
    return { term, fit };
  }

  async function newTab() {
    if (!isTauri()) return;
    try {
      loading = true;
      const info = await invoke("spawn_terminal");
      loading = false;
      const id = info.id;
      const tab = {
        id,
        label: "shell",
        labelFull: "",
        term: null,
        fit: null,
        pending: true,
        container: null,
      };
      tabs = [...tabs, tab];
      activeTabId = id;
      syncTabs();
      // update label to HOME path shortly after
      setTimeout(() => {
        const tb = tabs.find((x) => x.id === id);
        if (tb) {
          tb.label = "~";
          tb.labelFull = "~";
        }
        syncTabs();
      }, 300);
      return id;
    } catch (e) {
      toast(`Terminal error: ${e}`, "error");
      return null;
    }
  }

  function closeTab(id) {
    const idx = tabs.findIndex((x) => x.id === id);
    if (idx === -1) return;
    const tab = tabs[idx];
    tab.term?.dispose();
    if (isTauri()) invoke("kill_terminal", { id });
    tabs = tabs.filter((x) => x.id !== id);
    if (activeTabId === id) {
      activeTabId = tabs.length ? tabs[tabs.length - 1].id : null;
    }
    syncTabs();
    if (!tabs.length) refreshWorkspace();
  }

  function activateTab(id) {
    activeTabId = id;
    syncTabs();
    // refit
    setTimeout(() => {
      const tab = tabs.find((x) => x.id === id);
      tab?.fit?.fit();
    }, 30);
  }

  function syncTabs() {
    terminalTabs.set(
      tabs.map((x) => ({ id: x.id, label: x.label, labelFull: x.labelFull, active: x.id === activeTabId }))
    );
    // expose active terminal id for AI insert
    if (typeof window !== "undefined") window.__activeTerminalId = activeTabId;
    // mount terms after DOM update
    setTimeout(mountTerms, 20);
  }

  function mountTerms() {
    for (const tab of tabs) {
      const el = containerRefs[tab.id];
      if (el && !tab.term) {
        const { term, fit } = createTerm();
        term.open(el);
        term.writeln("\x1b[36m~ cozy-works terminal\x1b[0m");
        tab.term = term;
        tab.fit = fit;
        fit.fit();
        // wire data
        term.onData((data) => {
          if (!activeTabId) return;
          invoke("write_terminal", { id: tab.id, data }).catch(() => {});
          if (data === "\r") parseOutput(tab, "\r");
        });
        if (tab.pending) {
          tab.pending = false;
          term.write("");
        }
      }
      if (el) el.style.display = tab.id === activeTabId ? "block" : "none";
      if (tab.id === activeTabId) {
        setTimeout(() => tab.fit?.fit(), 50);
      }
    }
  }

  // Svelte action to register container element per tab id
  function containerAction(node, tabId) {
    containerRefs[tabId] = node;
    return {
      destroy() {
        delete containerRefs[tabId];
      },
    };
  }

  function parseOutput(tab, data) {
    // extract cwd from prompt lines like "user@host:/path$"
    const lines = String(data).split("\n");
    const last = lines[lines.length - 1] || "";
    const m = last.match(/([^\s]*\/[^\s$]*)\s*[\$#]/);
    if (m) {
      const dir = m[1];
      if (tab.label !== "~") {
        tab.label = basename(dir);
        tab.labelFull = dir;
      }
      syncTabs();
      refreshWorkspace(dir);
    }
  }

  function basename(path) {
    const cleaned = path.replace(/\\/g, "/").replace(/\/+$/, "");
    const parts = cleaned.split("/");
    return parts[parts.length - 1] || path;
  }

  async function refreshWorkspace(dir) {
    if (!isTauri()) return;
    const active = tabs.find((x) => x.id === activeTabId);
    const cwd = dir || active?.label || "~";
    try {
      const r = await invoke("git_branch", { cwd });
      const ws = get(workspaceStatus);
      workspaceStatus.set({
        branch: r.branch,
        dirty: r.dirty,
        dir: r.dir,
        aiRunning: ws.aiRunning,
        aiAction: ws.aiAction,
      });
      if (active && r.dir) {
        active.label = basename(r.dir);
        active.labelFull = r.dir;
        syncTabs();
      }
    } catch (e) {
      /* not a git repo */
    }
  }

  // listen PTY events for ALL tabs
  async function initListeners() {
    unlisten.push(
      await listen("pty://output", (e) => {
        const { id, data } = e.payload;
        const tab = tabs.find((x) => x.id === id);
        if (tab && tab.term) {
          tab.term.write(data);
          if (id === activeTabId) parseOutput(tab, data);
        }
      })
    );
    unlisten.push(
      await listen("pty://exit", (e) => {
        const { id } = e.payload;
        const tab = tabs.find((x) => x.id === id);
        if (tab && tab.term) {
          tab.term.writeln("\r\n\x1b[33m[process exited]\x1b[0m");
        }
      })
    );
  }

  onMount(async () => {
    if (isTauri()) {
      await initListeners();
      await newTab();
    } else {
      // browser fallback: fake terminal
      const id = "browser";
      tabs = [{ id, label: "demo", term: null, fit: null, pending: false, container: null }];
      activeTabId = id;
      syncTabs();
      setTimeout(() => {
        const tab = tabs[0];
        if (tab && tab.term) {
          tab.term.writeln("\x1b[36mCozy Works terminal — browser demo mode\x1b[0m");
          tab.term.onData((d) => {
            tab.term.write(d);
            if (d === "\r") tab.term.write("\r\n$ ");
          });
        }
      }, 100);
    }
    // expose focus function for sidebar agent click (App.svelte)
    window.__focusTerminalTab = (termId) => {
      if (tabs.some((x) => x.id === termId)) {
        activateTab(termId);
      }
    };
    window.addEventListener("resize", () => {
      const active = tabs.find((x) => x.id === activeTabId);
      active?.fit?.fit();
    });
  });

  onDestroy(() => {
    unlisten.forEach((u) => u());
    for (const tab of tabs) {
      tab.term?.dispose();
      if (isTauri()) invoke("kill_terminal", { id: tab.id }).catch(() => {});
    }
  });
</script>

<div class="terminal-view">
  <div class="tab-bar">
    {#each tabs as tab}
      <div class="term-tab" class:active={tab.id === activeTabId} onclick={() => activateTab(tab.id)} title={tab.labelFull || tab.label}>
        <Icon name="terminal" size={12} />
        <span class="tab-label">{tab.label}</span>
        <button class="tab-close" onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }}>
          <Icon name="close" size={10} />
        </button>
      </div>
    {/each}
    <button class="tab-add" onclick={newTab} title={$t.terminal.newTab}>
      <Icon name="plus" size={13} />
    </button>
    <span class="tab-spacer"></span>
    <span class="tab-hint">git branch shown in topbar</span>
  </div>

  <div class="term-container">
    {#if loading}
      <div class="term-loading">
        <span class="pixel-spinner"></span>
        <span>Starting shell…</span>
      </div>
    {/if}
    {#each tabs as tab}
      <div class="term-pane" use:containerAction={tab.id}></div>
    {/each}
  </div>
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
  }
  .tab-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    background: var(--surface-container-low);
    border-bottom: 2px solid var(--text);
  }
  .term-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: var(--surface);
    border: 2px solid var(--border);
    border-radius: 4px 4px 0 0;
    color: var(--text-dim);
    cursor: pointer;
    font-family: var(--font-body);
    font-size: 14px;
    max-width: 180px;
  }
  .term-tab.active {
    background: var(--primary);
    border-color: var(--text);
    color: var(--on-primary);
    font-weight: 700;
  }
  .tab-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }
  .tab-close {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    color: inherit;
    opacity: 0.6;
  }
  .tab-close:hover {
    opacity: 1;
    background: rgba(0, 0, 0, 0.1);
  }
  .tab-add {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
    background: var(--surface);
    border: 2px solid var(--border);
  }
  .tab-add:hover {
    background: var(--primary);
    color: var(--on-primary);
    border-color: var(--text);
  }
  .tab-spacer {
    flex: 1;
  }
  .tab-hint {
    font-family: var(--font-body);
    font-size: 12px;
    color: var(--text-faint);
  }
  .term-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .term-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    font-family: var(--font-body);
    font-size: 16px;
    color: var(--text-dim);
    background: var(--surface);
    z-index: 10;
  }
  .pixel-spinner {
    width: 16px;
    height: 16px;
    border: 3px solid var(--border);
    border-top-color: var(--primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .term-pane {
    position: absolute;
    inset: 0;
    padding: 4px;
    overflow: hidden;
  }
  :global(.xterm) {
    height: 100%;
  }
</style>
