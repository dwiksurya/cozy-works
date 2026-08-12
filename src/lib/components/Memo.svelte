<script>
  import { memos, toast } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";
  import Icon from "./Icon.svelte";

  let memoText = "";

  const dbPath = "sqlite:cozy.db";

  async function loadMemos() {
    try {
      const db = await Database.load(dbPath);
      const rows = await db.select("SELECT * FROM memos ORDER BY updated_at DESC LIMIT 30");
      memos.set(rows);
    } catch (e) {
      console.warn("loadMemos", e);
    }
  }

  async function saveMemo() {
    const text = memoText.trim();
    if (!text) return;
    try {
      const db = await Database.load(dbPath);
      await db.execute("INSERT INTO memos (content) VALUES ($1)", [text]);
      memoText = "";
      loadMemos();
    } catch (e) {
      toast(e, "error");
    }
  }

  async function deleteMemo(id) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("DELETE FROM memos WHERE id = $1", [id]);
      loadMemos();
    } catch (e) {
      console.warn(e);
    }
  }

  onMount(loadMemos);
</script>

<div class="memo-view">
  <h2>{$t.memo.title}</h2>
  <div class="memo-input pixel-panel">
    <textarea bind:value={memoText} placeholder={$t.memo.placeholder}></textarea>
    <button class="pixel-btn primary" onclick={saveMemo}>{$t.memo.save}</button>
  </div>

  <div class="memo-grid">
    {#each $memos as m}
      <div class="memo-card pixel-panel">
        <p>{m.content}</p>
        <button class="mini danger" onclick={() => deleteMemo(m.id)}><Icon name="trash" size={13} /></button>
      </div>
    {:else}
      <div class="empty">—</div>
    {/each}
  </div>
</div>

<style>
  .memo-view {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
  }
  .memo-view h2 {
    font-size: 20px;
    margin-bottom: 14px;
  }
  .memo-input {
    padding: 14px;
    margin-bottom: 16px;
    display: flex;
    gap: 10px;
    flex-direction: column;
  }
  .memo-input textarea {
    min-height: 80px;
    resize: vertical;
  }
  .memo-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }
  .memo-card {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: #232838;
    border-color: #3a3f52;
    transform: rotate(-0.5deg);
  }
  .memo-card p {
    font-size: 13px;
    line-height: 1.5;
    user-select: text;
    word-break: break-word;
  }
  .mini {
    align-self: flex-end;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-faint);
  }
  .mini:hover {
    color: var(--danger);
  }
  .empty {
    color: var(--text-faint);
    text-align: center;
    padding: 40px;
    grid-column: 1 / -1;
  }
</style>
