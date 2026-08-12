<script>
  import { pomodoro, todos, memos, activeView } from "../stores.js";
  import { t, lang } from "../i18n-store.js";
  import { fmtTime, todayStr } from "../i18n-store.js";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";

  let stats = { sessions: 0, minutes: 0 };

  onMount(async () => {
    try {
      const db = await Database.load("sqlite:cozy.db");
      const rows = await db.select("SELECT * FROM pomodoro_stats WHERE date = $1", [todayStr()]);
      if (rows.length) {
        stats = { sessions: rows[0].sessions, minutes: rows[0].focus_minutes };
      }
    } catch (e) {
      /* ignore */
    }
  });

  function activeTodoCount() {
    return $todos.filter((x) => x.status !== "done").length;
  }
</script>

<div class="dashboard">
  <div class="dash-hero">
    <div class="hero-left">
      <h1>{$t.dashboard.greeting} 👋</h1>
      <p class="hero-sub">{new Date().toLocaleDateString($lang === "id" ? "id-ID" : "en-US", { weekday: "long", month: "long", day: "numeric" })}</p>
      <div class="hero-stats">
        <div class="hero-stat">
          <span class="hs-num">{$pomodoro.completedToday}</span>
          <span class="hs-label">{$t.pomodoro.completed}</span>
        </div>
        <div class="hero-stat">
          <span class="hs-num">{$pomodoro.todayMinutes}m</span>
          <span class="hs-label">{$t.pomodoro.focusTime}</span>
        </div>
        <div class="hero-stat">
          <span class="hs-num">{activeTodoCount()}</span>
          <span class="hs-label">{$t.dashboard.activeTodo}</span>
        </div>
      </div>
      <button class="pixel-btn primary hero-btn" onclick={() => activeView.set("pomodoro")}>
        {$t.dashboard.startFocus} ▶
      </button>
    </div>
  </div>

  <div class="dash-grid">
    <div class="dash-card pixel-panel">
      <div class="dash-card-head">
        <span>{$t.nav.todo}</span>
        <button class="link-btn" onclick={() => activeView.set("todo")}>→</button>
      </div>
      <div class="mini-todo">
        {#each $todos.filter((x) => x.status !== "done").slice(0, 5) as td}
          <div class="mini-todo-item">
            <span class="dot" style="background: {td.priority === 'high' ? 'var(--danger)' : td.priority === 'medium' ? 'var(--warn)' : 'var(--accent)'}"></span>
            <span>{td.title}</span>
          </div>
        {:else}
          <div class="empty-mini">{$t.todo.empty}</div>
        {/each}
      </div>
    </div>

    <div class="dash-card pixel-panel">
      <div class="dash-card-head">
        <span>{$t.nav.memo}</span>
        <button class="link-btn" onclick={() => activeView.set("memo")}>→</button>
      </div>
      <div class="mini-memo">
        {#each $memos.slice(0, 3) as m}
          <div class="mini-memo-item">{m.content}</div>
        {:else}
          <div class="empty-mini">—</div>
        {/each}
      </div>
    </div>

    <div class="dash-card pixel-panel">
      <div class="dash-card-head">
        <span>{$t.pomodoro.stats}</span>
      </div>
      <div class="mini-focus">
        <div class="mf-ring" style="--pct: {Math.min(100, Math.round((stats.minutes / 120) * 100))}%">
          <span class="mf-num">{stats.minutes}m</span>
        </div>
        <p>{stats.sessions} {$t.pomodoro.sessions}</p>
      </div>
    </div>
  </div>
</div>

<style>
  .dashboard {
    padding: 24px;
    overflow-y: auto;
    height: 100%;
  }
  .dash-hero {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-panel);
    border: 1px solid var(--border-soft);
    border-radius: var(--radius);
    padding: 24px 28px;
    margin-bottom: 18px;
    box-shadow: var(--shadow);
  }
  .hero-left h1 {
    font-size: 22px;
    margin-bottom: 4px;
  }
  .hero-sub {
    color: var(--text-dim);
    font-size: 13px;
    margin-bottom: 16px;
  }
  .hero-stats {
    display: flex;
    gap: 24px;
    margin-bottom: 18px;
  }
  .hero-stat {
    display: flex;
    flex-direction: column;
  }
  .hs-num {
    font-size: 24px;
    font-weight: 400;
    color: var(--checklist-green);
    font-family: var(--font-timer);
  }
  .hs-label {
    font-size: 11px;
    color: var(--text-faint);
  }
  .hero-btn {
    padding: 10px 20px;
  }
  .hero-right {
    display: flex;
    gap: 14px;
    align-items: flex-end;
  }
  .dash-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 14px;
  }
  .dash-card {
    padding: 14px;
  }
  .dash-card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-dim);
    margin-bottom: 10px;
  }
  .link-btn {
    color: var(--text-faint);
    font-size: 14px;
  }
  .mini-todo {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mini-todo-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .mini-memo {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mini-memo-item {
    font-size: 12.5px;
    color: var(--text-dim);
    background: var(--bg-elev);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    border-left: 3px solid var(--accent-dim);
    user-select: text;
  }
  .mini-focus {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .mf-ring {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: conic-gradient(var(--accent) var(--pct), var(--border) 0);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .mf-ring .mf-num {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--surface-container-low);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 400;
    font-family: var(--font-timer);
  }
  .mini-focus p {
    color: var(--text-dim);
    font-size: 13px;
  }
  .empty-mini {
    color: var(--text-faint);
    font-size: 12px;
    padding: 10px 0;
  }
</style>
