<script>
  import "./app.css";
  import { activeView, settings, settingsLoaded, sidebarCollapsed, workspaceStatus, agents, toast } from "./lib/stores.js";
  import { t, lang } from "./lib/i18n-store.js";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import Dashboard from "./lib/components/Dashboard.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import Todo from "./lib/components/Todo.svelte";
  import Notes from "./lib/components/Notes.svelte";
  import Memo from "./lib/components/Memo.svelte";
  import Music from "./lib/components/Music.svelte";
  import Terminal from "./lib/components/Terminal.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import Icon from "./lib/components/Icon.svelte";

  let appWindow = null;
  let sidebarWidth = 220; // px, min = 220 (current), max = 320

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  async function loadSettings() {
    if (!isTauri()) {
      settingsLoaded.set(true);
      return;
    }
    try {
      const db = await Database.load("sqlite:cozy.db");
      const rows = await db.select("SELECT key, value FROM settings");
      const obj = {};
      for (const r of rows) obj[r.key] = r.value;
      settings.update((s) => ({ ...s, ...obj }));
      if (obj.lang) lang.set(obj.lang);
      if (obj.sidebarCollapsed === "true") sidebarCollapsed.set(true);
      if (obj.sidebarWidth) {
        const w = parseInt(obj.sidebarWidth, 10);
        if (w >= 220 && w <= 320) sidebarWidth = w;
      }
    } catch (e) {
      console.warn("loadSettings", e);
    }
    settingsLoaded.set(true);
  }

  onMount(async () => {
    await loadSettings();
    if (isTauri()) {
      try {
        appWindow = getCurrentWindow();
      } catch (e) {
        console.warn("window init", e);
      }
    }
    const unsub = settings.subscribe((s) => {
      if (s.lang) lang.set(s.lang);
    });
    // persist sidebar collapse
    const unsub2 = sidebarCollapsed.subscribe((collapsed) => {
      if (isTauri()) {
        try {
          Database.load("sqlite:cozy.db").then((db) => {
            db.execute("INSERT INTO settings (key, value) VALUES ('sidebarCollapsed', $1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [String(collapsed)]);
          });
        } catch (e) {
          /* ignore */
        }
      }
    });
    // auto-expand sidebar when AI needs confirmation (decision 4b)
    const unsub3 = workspaceStatus.subscribe((ws) => {
      if (ws.aiAction === "needs-confirm" && $sidebarCollapsed) {
        sidebarCollapsed.set(false);
      }
    });
    // poll agents every 3s
    if (isTauri()) {
      pollAgents();
      agentsTimer = setInterval(pollAgents, 3000);
    }
    return () => {
      unsub();
      unsub2();
      unsub3();
      if (agentsTimer) clearInterval(agentsTimer);
    };
  });

  function minimize() {
    appWindow?.minimize();
  }
  function toggleMaximize() {
    appWindow?.toggleMaximize();
  }
  function closeApp() {
    appWindow?.close();
  }

  // ---- agents polling ----
  let agentsTimer;
  async function pollAgents() {
    if (!isTauri()) return;
    try {
      const list = await window.__TAURI_INTERNALS__.invoke("list_agents");
      agents.set(list);
    } catch (e) {
      /* backend not ready */
    }
  }
  function focusAgentTerminal(terminalId) {
    // switch to terminal view + activate that tab
    activeView.set("terminal");
    window.__focusTerminalTab?.(terminalId);
  }
  // folder basename
  function wsName(full) {
    if (!full) return "";
    const cleaned = String(full).replace(/\\/g, "/").replace(/\/+$/, "");
    const parts = cleaned.split("/");
    return parts[parts.length - 1] || cleaned;
  }

  const navItems = [
    { id: "dashboard", icon: "home", key: "nav.dashboard" },
    { id: "pomodoro", icon: "clock", key: "nav.pomodoro" },
    { id: "todo", icon: "check-double", key: "nav.todo" },
    { id: "notes", icon: "doc", key: "nav.notes" },
    { id: "memo", icon: "note", key: "nav.memo" },
    { id: "music", icon: "music", key: "nav.music" },
    { id: "terminal", icon: "terminal", key: "nav.terminal" },
  ];

  function label(key) {
    return key.split(".").reduce((o, k) => (o ? o[k] : ""), $t);
  }

  function navClass(id) {
    return $activeView === id ? "nav-item active" : "nav-item";
  }

  // ---- sidebar resize (220–320px) ----
  function onResizeStart(e) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidth;
    function onMove(ev) {
      const w = Math.min(320, Math.max(220, startW + (ev.clientX - startX)));
      sidebarWidth = w;
    }
    function onUp() {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      if (isTauri()) {
        Database.load("sqlite:cozy.db").then((db) => {
          db.execute("INSERT INTO settings (key, value) VALUES ('sidebarWidth', $1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [String(sidebarWidth)]);
        });
      }
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }
</script>

<div class="app-shell">
  <!-- ===== Custom titlebar (drag region) ===== -->
  <div class="titlebar" data-tauri-drag-region>
    <div class="tb-left" data-tauri-drag-region>
      <img src="logo.png" alt="" class="tb-logo pixel-canvas" data-tauri-drag-region />
      <span class="tb-title" data-tauri-drag-region>{$t.brand}</span>
    </div>

    <div class="tb-center" data-tauri-drag-region>
      {#if $workspaceStatus.branch}
        <span class="tb-chip branch"><Icon name="check" size={11} /> {$workspaceStatus.branch}{#if $workspaceStatus.dirty} •{/if}</span>
      {/if}
      {#if $workspaceStatus.dir}
        <span class="tb-chip dir"><Icon name="terminal" size={11} /> {$workspaceStatus.dir}</span>
      {/if}
    </div>

    <div class="tb-right">
      <button class="tb-btn" onclick={() => activeView.set("settings")} class:active={$activeView === "settings"} title={$t.nav.settings}>
        <Icon name="settings" size={15} />
      </button>
      <button class="tb-btn" onclick={minimize} title="minimize">
        <Icon name="chevron-down" size={15} />
      </button>
      <button class="tb-btn" onclick={toggleMaximize} title="maximize">
        <Icon name="tab-plus" size={15} />
      </button>
      <button class="tb-btn close" onclick={closeApp} title="close">
        <Icon name="close" size={15} />
      </button>
    </div>
  </div>

  <div class="app-body">
    <aside class="sidebar" class:collapsed={$sidebarCollapsed} style={$sidebarCollapsed ? "" : `width: ${sidebarWidth}px`}>
      <button class="collapse-btn" onclick={() => sidebarCollapsed.set(!$sidebarCollapsed)} title="toggle sidebar">
        <Icon name={$sidebarCollapsed ? "arrow-right" : "arrow-left"} size={14} />
      </button>

      {#if !$sidebarCollapsed}
        <div class="brand">
          <img src="logo.png" alt="" class="logo pixel-canvas" />
          <span>{$t.brand}</span>
        </div>
      {:else}
        <div class="brand-collapsed">
          <img src="logo.png" alt="" class="logo pixel-canvas" />
        </div>
      {/if}

      {#each navItems as item}
        <button class={navClass(item.id)} onclick={() => (activeView.set(item.id))} title={$sidebarCollapsed ? label(item.key) : ""}>
          <span class="nav-icon"><Icon name={item.icon} size={17} /></span>
          {#if !$sidebarCollapsed}
            <span>{label(item.key)}</span>
          {/if}
        </button>
      {/each}

      <!-- workspace status -->
      <div class="sidebar-section" class:collapsed={$sidebarCollapsed}>
        {#if !$sidebarCollapsed}
          <div class="section-label">WORKSPACE</div>
        {/if}
        <div class="ws-row">
          <Icon name="terminal" size={12} />
          {#if !$sidebarCollapsed}
            <span class="ws-folder">{wsName($workspaceStatus.dir) || "~"}</span>
            {#if $workspaceStatus.branch}
              <span class="ws-branch">{#if $workspaceStatus.dirty}*{/if}{$workspaceStatus.branch}</span>
            {/if}
          {:else}
            <span class="ws-dot" title={$workspaceStatus.dir || "~"}></span>
          {/if}
        </div>
      </div>

      <!-- agents -->
      <div class="sidebar-section agents" class:collapsed={$sidebarCollapsed}>
        {#if !$sidebarCollapsed}
          <div class="section-label">AGENTS {$agents.length || ""}</div>
        {/if}
        {#each $agents as agent}
          <button
            class="agent-item"
            class:collapsed={$sidebarCollapsed}
            onclick={() => focusAgentTerminal(agent.terminal_id)}
            title={$sidebarCollapsed ? `${agent.name}: ${agent.status}` : `${agent.name} — ${agent.status}${agent.title ? ` · ${agent.title}` : ""} — click to focus terminal`}
          >
            <span class="agent-dot" class:running={agent.status === "running"} class:blocker={agent.status === "blocker"} class:idle={agent.status === "idle"}></span>
            {#if !$sidebarCollapsed}
              <span class="agent-name">{agent.name}</span>
              <span class="agent-status" class:running={agent.status === "running"} class:blocker={agent.status === "blocker"} class:idle={agent.status === "idle"}>{agent.status}</span>
            {/if}
          </button>
        {/each}
        {#if !$agents.length && !$sidebarCollapsed}
          <div class="agents-empty">no agents running</div>
        {/if}
      </div>

      <div class="sidebar-footer">
        <button class="nav-item" onclick={() => activeView.set("settings")} title={$sidebarCollapsed ? $t.nav.settings : ""}>
          <span class="nav-icon"><Icon name="settings" size={17} /></span>
          {#if !$sidebarCollapsed}
            <span>{$t.nav.settings}</span>
          {/if}
        </button>
      </div>
      {#if !$sidebarCollapsed}
        <div class="sidebar-resizer" onmousedown={onResizeStart} title="drag to resize"></div>
      {/if}
    </aside>

    <main class="main-area">
      {#if $activeView === "dashboard"}
        <Dashboard />
      {:else if $activeView === "pomodoro"}
        <Pomodoro />
      {:else if $activeView === "todo"}
        <Todo />
      {:else if $activeView === "notes"}
        <Notes />
      {:else if $activeView === "memo"}
        <Memo />
      {:else if $activeView === "music"}
        <Music />
      {:else if $activeView === "terminal"}
        <Terminal />
      {:else if $activeView === "settings"}
        <Settings />
      {/if}
    </main>
  </div>

  <Toast />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: var(--bg);
  }

  /* ===== Custom titlebar ===== */
  .titlebar {
    display: flex;
    align-items: center;
    height: 36px;
    flex-shrink: 0;
    background: var(--surface-container-low);
    border-bottom: 2px solid var(--text);
    padding: 0 6px;
    user-select: none;
    -webkit-app-region: drag;
  }
  .titlebar button {
    -webkit-app-region: no-drag;
  }
  .tb-left {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 8px;
  }
  .tb-logo {
    width: 16px;
    height: 16px;
    image-rendering: pixelated;
  }
  .tb-title {
    font-family: var(--font-heading);
    font-weight: 700;
    font-size: 13px;
    color: var(--text);
  }
  .tb-center {
    flex: 1;
    display: flex;
    gap: 6px;
    justify-content: center;
    align-items: center;
    overflow: hidden;
    padding: 0 10px;
  }
  .tb-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-body);
    font-size: 13px;
    padding: 1px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-dim);
    max-width: 180px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tb-chip.branch {
    color: var(--checklist-green);
  }
  .tb-chip.ai {
    color: var(--music-purple);
  }
  .tb-right {
    display: flex;
    gap: 2px;
    -webkit-app-region: no-drag;
  }
  .tb-btn {
    width: 30px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 14px;
  }
  .tb-btn:hover {
    background: var(--surface-dim);
    color: var(--text);
  }
  .tb-btn.active {
    background: var(--primary);
    color: var(--on-primary);
  }
  .tb-btn.close:hover {
    background: var(--pomodoro-red);
    color: #fff;
  }

  /* ===== Body (below titlebar) ===== */
  .app-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .sidebar {
    width: 220px;
    flex-shrink: 0;
    background: var(--surface-container-low);
    border-right: 2px solid var(--text);
    display: flex;
    flex-direction: column;
    padding: 10px 10px;
    gap: 3px;
    transition: width 0.15s ease;
    position: relative;
  }
  .sidebar.collapsed {
    width: 56px;
    padding: 10px 6px;
  }
  .collapse-btn {
    position: absolute;
    top: 8px;
    right: 6px;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-faint);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .collapse-btn:hover {
    background: var(--surface-dim);
    color: var(--text);
  }
  .sidebar.collapsed .collapse-btn {
    right: auto;
    left: 50%;
    transform: translateX(-50%);
  }

  .sidebar-resizer {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    z-index: 30;
  }
  .sidebar-resizer:hover {
    background: var(--primary);
    opacity: 0.5;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px 14px;
    font-family: var(--font-heading);
    font-weight: 700;
    font-size: 15px;
    color: var(--text);
    border-bottom: 2px dashed var(--border);
    margin-bottom: 8px;
  }
  .brand .logo,
  .brand-collapsed .logo {
    width: 22px;
    height: 22px;
    image-rendering: pixelated;
  }
  .brand-collapsed {
    display: flex;
    justify-content: center;
    padding: 6px 0 12px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    font-family: var(--font-menu);
    font-size: 12px;
    letter-spacing: 0.5px;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
    border: 2px solid transparent;
    background: none;
    text-align: left;
    width: 100%;
  }
  .sidebar.collapsed .nav-item {
    justify-content: center;
    padding: 8px 0;
  }
  .nav-item:hover {
    background: var(--surface);
    color: var(--text);
    border-color: var(--border);
  }
  .nav-item.active {
    background: var(--primary);
    border-color: var(--text);
    color: var(--on-primary);
    font-weight: 700;
    box-shadow: 2px 2px 0 rgba(61, 50, 38, 0.2);
  }
  .nav-icon {
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .sidebar-section {
    margin-top: 10px;
    padding-top: 8px;
    border-top: 2px dashed var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .sidebar-section.collapsed {
    align-items: center;
  }
  .section-label {
    font-family: var(--font-menu);
    font-size: 10px;
    letter-spacing: 1px;
    color: var(--text-faint);
    padding: 0 4px;
  }
  .ws-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-body);
    font-size: 14px;
    color: var(--checklist-green);
    padding: 2px 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 3px;
    white-space: nowrap;
    overflow: hidden;
  }
  .ws-folder {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 90px;
  }
  .ws-branch {
    color: var(--text-dim);
    font-size: 12px;
  }
  .ws-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--checklist-green);
    display: inline-block;
  }
  .sidebar-section.agents {
    gap: 2px;
  }
  .agent-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: 3px;
    font-family: var(--font-body);
    font-size: 13px;
    color: var(--text-dim);
    border: 1px solid transparent;
    background: none;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }
  .agent-item:hover {
    background: var(--surface);
    border-color: var(--border);
    color: var(--text);
  }
  .agent-item.collapsed {
    justify-content: center;
    padding: 3px 0;
  }
  .agent-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-faint);
    flex-shrink: 0;
  }
  .agent-dot.running {
    background: var(--checklist-green);
  }
  .agent-dot.blocker {
    background: var(--danger);
  }
  .agent-dot.idle {
    background: var(--accent);
    opacity: 0.7;
  }
  .agent-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .agent-status {
    font-size: 11px;
    color: var(--text-faint);
    text-transform: lowercase;
  }
  .agent-status.running {
    color: var(--checklist-green);
  }
  .agent-status.blocker {
    color: var(--danger);
    font-weight: 700;
  }
  .agent-status.idle {
    color: var(--accent);
    opacity: 0.8;
  }
  .agents-empty {
    font-family: var(--font-body);
    font-size: 12px;
    color: var(--text-faint);
    padding: 2px 6px;
  }

  .sidebar-footer {
    margin-top: auto;
    padding-top: 10px;
    border-top: 2px dashed var(--border);
  }
  .sidebar.collapsed .sidebar-footer {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .sidebar.collapsed .sidebar-footer .nav-item {
    width: auto;
  }

  .main-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
    min-width: 0;
  }
</style>
