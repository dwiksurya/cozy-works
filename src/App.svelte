<script>
  import "./app.css";
  import { activeView, showAiSidebar, settings, settingsLoaded, toast } from "./lib/stores.js";
  import { t, lang } from "./lib/i18n-store.js";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";

  import Dashboard from "./lib/components/Dashboard.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import Todo from "./lib/components/Todo.svelte";
  import Notes from "./lib/components/Notes.svelte";
  import Memo from "./lib/components/Memo.svelte";
  import Music from "./lib/components/Music.svelte";
  import Terminal from "./lib/components/Terminal.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import AiPanel from "./lib/components/AiPanel.svelte";
  import Pet from "./lib/components/Pet.svelte";
  import Avatar from "./lib/components/Avatar.svelte";
  import AmbientDock from "./lib/components/AmbientDock.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import Icon from "./lib/components/Icon.svelte";

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
    } catch (e) {
      console.warn("loadSettings", e);
    }
    settingsLoaded.set(true);
  }

  onMount(() => {
    loadSettings();
    // keep lang in sync when settings.lang changes
    const unsub = settings.subscribe((s) => {
      if (s.lang) lang.set(s.lang);
    });
    return unsub;
  });

  const navItems = [
    { id: "dashboard", icon: "home", key: "nav.dashboard" },
    { id: "pomodoro", icon: "clock", key: "nav.pomodoro" },
    { id: "todo", icon: "check-double", key: "nav.todo" },
    { id: "notes", icon: "doc", key: "nav.notes" },
    { id: "memo", icon: "note", key: "nav.memo" },
    { id: "music", icon: "music", key: "nav.music" },
    { id: "terminal", icon: "terminal", key: "nav.terminal" },
    { id: "settings", icon: "settings", key: "nav.settings" },
  ];

  function label(key) {
    return key.split(".").reduce((o, k) => (o ? o[k] : ""), $t);
  }

  // pet & avatar always visible in sidebar footer
  const petAnimal = $settings.pet || "cat";
</script>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <img src="logo.png" alt="" class="logo pixel-canvas" />
      <span>{$t.brand}</span>
    </div>
    {#each navItems as item}
      <button class="nav-item" class:active={$activeView === item.id} onclick={() => (activeView.set(item.id))}>
        <span class="nav-icon"><Icon name={item.icon} size={17} /></span>
        <span>{label(item.key)}</span>
      </button>
    {/each}

    <div class="sidebar-footer">
      <div class="footer-row">
        <Avatar size={36} />
        <Pet size={40} showLabel={false} />
      </div>
      <button class="nav-item" class:active={$showAiSidebar} onclick={() => showAiSidebar.set(!$showAiSidebar)}>
        <span class="nav-icon"><Icon name="spark" size={17} /></span>
        <span>{$t.nav.ai}</span>
      </button>
    </div>
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

    <div class="pet-corner">
      <Pet size={64} />
    </div>

    <div class="ambient-dock">
      <AmbientDock />
    </div>
  </main>

  {#if $showAiSidebar}
    <AiPanel />
  {/if}

  <Toast />
</div>

<style>
  .footer-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
  }
</style>
