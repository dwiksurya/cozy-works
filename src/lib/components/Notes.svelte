<script>
  import { notes, toast } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { get } from "svelte/store";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";
  import { marked } from "marked";

  let search = "";
  let editing = null;
  let editTitle = "";
  let editContent = "";
  let editTags = "";
  let showArchived = false;
  let showPinnedOnly = false;

  const dbPath = "sqlite:cozy.db";

  async function loadNotes() {
    try {
      const db = await Database.load(dbPath);
      const rows = await db.select("SELECT * FROM notes ORDER BY pinned DESC, updated_at DESC");
      notes.set(rows);
    } catch (e) {
      console.warn("loadNotes", e);
    }
  }

  function newNote() {
    editing = { id: null, title: "", content: "", tags: "" };
    editTitle = "";
    editContent = "";
    editTags = "";
  }

  async function saveNote() {
    if (!editing) return;
    try {
      const db = await Database.load(dbPath);
      if (editing.id) {
        await db.execute("UPDATE notes SET title = $1, content = $2, tags = $3, updated_at = datetime('now') WHERE id = $4", [
          editTitle || "Untitled",
          editContent,
          editTags,
          editing.id,
        ]);
      } else {
        await db.execute("INSERT INTO notes (title, content, tags) VALUES ($1, $2, $3)", [
          editTitle || "Untitled",
          editContent,
          editTags,
        ]);
      }
      editing = null;
      loadNotes();
    } catch (e) {
      toast(e, "error");
    }
  }

  async function togglePin(n) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("UPDATE notes SET pinned = $1 WHERE id = $2", [n.pinned ? 0 : 1, n.id]);
      loadNotes();
    } catch (e) {
      console.warn(e);
    }
  }

  async function toggleArchive(n) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("UPDATE notes SET archived = $1 WHERE id = $2", [n.archived ? 0 : 1, n.id]);
      loadNotes();
    } catch (e) {
      console.warn(e);
    }
  }

  async function deleteNote(n) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("DELETE FROM notes WHERE id = $1", [n.id]);
      loadNotes();
    } catch (e) {
      console.warn(e);
    }
  }

  function openNote(n) {
    editing = n;
    editTitle = n.title;
    editContent = n.content;
    editTags = n.tags || "";
  }

  function renderMarkdown(src) {
    try {
      return marked.parse(src || "");
    } catch (e) {
      return src || "";
    }
  }

  function filtered() {
    let list = $notes;
    if (showArchived) list = list.filter((n) => n.archived);
    else list = list.filter((n) => !n.archived);
    if (showPinnedOnly) list = list.filter((n) => n.pinned);
    if (search) {
      const q = search.toLowerCase();
      list = list.filter((n) => (n.title + " " + n.content + " " + (n.tags || "")).toLowerCase().includes(q));
    }
    return list;
  }

  function tagsOf(n) {
    return (n.tags || "").split(",").map((s) => s.trim()).filter(Boolean);
  }

  function extractBacklinks(n) {
    // simple [[wiki]] backlinks
    const links = [...(n.content || "").matchAll(/\[\[([^\]]+)\]\]/g)].map((m) => m[1]);
    return links;
  }

  onMount(loadNotes);
</script>

<div class="notes-view">
  <div class="notes-header">
    <h2>{$t.notes.title}</h2>
    <div class="notes-actions">
      <input bind:value={search} placeholder={$t.notes.search} class="search" />
      <button class="pixel-btn" class:active={showPinnedOnly} onclick={() => (showPinnedOnly = !showPinnedOnly)}>📌</button>
      <button class="pixel-btn" class:active={showArchived} onclick={() => (showArchived = !showArchived)}>🗄</button>
      <button class="pixel-btn primary" onclick={newNote}>{$t.notes.newNote}</button>
    </div>
  </div>

  {#if editing}
    <div class="editor pixel-panel">
      <div class="editor-row">
        <input bind:value={editTitle} placeholder="Title" class="title-input" />
        <input bind:value={editTags} placeholder="tags, comma, separated" class="tags-input" />
        <button class="pixel-btn primary" onclick={saveNote}>💾</button>
        <button class="pixel-btn" onclick={() => (editing = null)}>✕</button>
      </div>
      <div class="editor-split">
        <textarea bind:value={editContent} class="content-input" placeholder="Write in markdown… [[link]] for backlink"></textarea>
        <div class="preview markdown-body">{@html renderMarkdown(editContent)}</div>
      </div>
    </div>
  {/if}

  <div class="notes-grid">
    {#each filtered() as n}
      <div class="note-card pixel-panel" class:pinned={!!n.pinned} onclick={() => openNote(n)}>
        <div class="note-card-top">
          <span class="note-title">{n.title || "Untitled"}</span>
          {#if n.pinned}<span class="pin-badge">📌</span>{/if}
        </div>
        {#if tagsOf(n).length}
          <div class="note-tags">
            {#each tagsOf(n) as tg}<span class="note-tag">#{tg}</span>{/each}
          </div>
        {/if}
        <div class="note-preview">{n.content?.slice(0, 100) || ""}</div>
        {#if extractBacklinks(n).length}
          <div class="note-backlinks">
            {#each extractBacklinks(n) as bl}<span class="bl">[[{bl}]]</span>{/each}
          </div>
        {/if}
        <div class="note-actions" onclick={(e) => e.stopPropagation()}>
          <button class="mini" onclick={() => togglePin(n)}>{n.pinned ? "📌" : "📍"}</button>
          <button class="mini" onclick={() => toggleArchive(n)}>🗄</button>
          <button class="mini danger" onclick={() => deleteNote(n)}>🗑</button>
        </div>
      </div>
    {:else}
      <div class="empty">{$t.notes.noNotes}</div>
    {/each}
  </div>
</div>

<style>
  .notes-view {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
  }
  .notes-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
    gap: 10px;
    flex-wrap: wrap;
  }
  .notes-header h2 {
    font-size: 20px;
  }
  .notes-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .search {
    width: 180px;
  }
  .pixel-btn.active {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent-dim);
  }
  .editor {
    margin-bottom: 16px;
    padding: 12px;
  }
  .editor-row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .title-input {
    flex: 2;
    font-size: 15px;
    font-weight: 600;
  }
  .tags-input {
    flex: 1;
  }
  .editor-split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .content-input {
    height: 200px;
    resize: vertical;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .preview {
    height: 200px;
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 12.5px;
    line-height: 1.5;
    user-select: text;
  }
  .markdown-body h1, .markdown-body h2 { margin: 4px 0; }
  .markdown-body p { margin: 4px 0; }
  .markdown-body code { background: var(--bg-elev); padding: 1px 4px; border-radius: 3px; }
  .markdown-body pre { background: var(--bg-elev); padding: 8px; border-radius: 6px; overflow-x: auto; }
  .notes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }
  .note-card {
    padding: 12px;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .note-card:hover {
    border-color: var(--text-faint);
  }
  .note-card.pinned {
    border-color: var(--accent-dim);
  }
  .note-card-top {
    display: flex;
    justify-content: space-between;
    gap: 6px;
  }
  .note-title {
    font-weight: 600;
    font-size: 13.5px;
  }
  .pin-badge {
    font-size: 12px;
  }
  .note-tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .note-tag {
    font-size: 10px;
    color: var(--blue);
  }
  .note-preview {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-dim);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    user-select: text;
  }
  .note-backlinks {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .bl {
    font-size: 10px;
    color: var(--purple);
  }
  .note-actions {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }
  .mini {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .mini.danger:hover {
    color: var(--danger);
  }
  .empty {
    grid-column: 1 / -1;
    color: var(--text-faint);
    text-align: center;
    padding: 40px;
  }
</style>
