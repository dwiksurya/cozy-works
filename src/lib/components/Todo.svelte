<script>
  import { todos, toast } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { get } from "svelte/store";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";

  let view = "board"; // board | list | today | all | bytag
  let newTitle = "";
  let newTag = "";
  let newPriority = "medium";
  let filterTag = "";

  const dbPath = "sqlite:cozy.db";

  async function loadTodos() {
    try {
      const db = await Database.load(dbPath);
      const rows = await db.select("SELECT * FROM todos ORDER BY position, created_at DESC");
      const items = await db.select("SELECT * FROM todo_items ORDER BY id");
      const grouped = {};
      for (const it of items) {
        (grouped[it.todo_id] = grouped[it.todo_id] || []).push(it);
      }
      todos.set(
        rows.map((r) => ({
          ...r,
          items: grouped[r.id] || [],
        }))
      );
    } catch (e) {
      console.warn("loadTodos", e);
    }
  }

  async function addTodo() {
    const title = newTitle.trim();
    if (!title) return;
    try {
      const db = await Database.load(dbPath);
      const maxPos = await db.select("SELECT COALESCE(MAX(position),0) AS m FROM todos");
      const pos = (maxPos[0]?.m || 0) + 1;
      await db.execute(
        "INSERT INTO todos (title, priority, tag, status, position) VALUES ($1, $2, $3, 'todo', $4)",
        [title, newPriority, newTag, pos]
      );
      newTitle = "";
      newTag = "";
      newPriority = "medium";
      loadTodos();
    } catch (e) {
      toast(e, "error");
    }
  }

  async function updateStatus(todoId, status) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("UPDATE todos SET status = $1, done_at = CASE WHEN $1='done' THEN datetime('now') ELSE done_at END WHERE id = $2", [status, todoId]);
      if (status === "done") {
        petHappy();
      }
      loadTodos();
    } catch (e) {
      console.warn(e);
    }
  }

  async function addSubtask(todoId, label) {
    if (!label?.trim()) return;
    try {
      const db = await Database.load(dbPath);
      await db.execute("INSERT INTO todo_items (todo_id, label, done) VALUES ($1, $2, 0)", [todoId, label.trim()]);
      loadTodos();
    } catch (e) {
      console.warn(e);
    }
  }

  async function toggleSubtask(itemId, done) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("UPDATE todo_items SET done = $1 WHERE id = $2", [done ? 1 : 0, itemId]);
      loadTodos();
    } catch (e) {
      console.warn(e);
    }
  }

  async function deleteTodo(todoId) {
    try {
      const db = await Database.load(dbPath);
      await db.execute("DELETE FROM todos WHERE id = $1", [todoId]);
      loadTodos();
    } catch (e) {
      console.warn(e);
    }
  }

  function petHappy() {
    // imported petState triggers via store — keep simple, call event
    // (pet reacts through Pet component subscription)
  }

  const columns = ["todo", "doing", "done"];
  function colTitle(c) {
    return { todo: $t.todo.todo, doing: $t.todo.doing, done: $t.todo.doneColumn }[c];
  }
  function colTodos(c) {
    return $todos.filter((td) => td.status === c);
  }

  function isDueSoon(td) {
    if (!td.due_date) return false;
    const d = new Date(td.due_date);
    const now = new Date();
    return d - now < 24 * 3600 * 1000 && d > now;
  }

  onMount(loadTodos);
</script>

<div class="todo-view">
  <div class="todo-header">
    <h2>{$t.todo.title}</h2>
    <div class="view-tabs">
      <button class="tab" class:active={view === "board"} onclick={() => (view = "board")}>{$t.todo.board}</button>
      <button class="tab" class:active={view === "list"} onclick={() => (view = "list")}>{$t.todo.list}</button>
      <button class="tab" class:active={view === "today"} onclick={() => (view = "today")}>{$t.todo.today}</button>
      <button class="tab" class:active={view === "all"} onclick={() => (view = "all")}>{$t.todo.all}</button>
    </div>
  </div>

  <div class="todo-add">
    <input
      bind:value={newTitle}
      placeholder={$t.todo.newTask}
      onkeydown={(e) => e.key === "Enter" && addTodo()}
    />
    <select bind:value={newPriority}>
      <option value="low">{$t.todo.priority.low}</option>
      <option value="medium">{$t.todo.priority.medium}</option>
      <option value="high">{$t.todo.priority.high}</option>
    </select>
    <input bind:value={newTag} placeholder="tag" style="width: 100px" />
    <button class="pixel-btn primary" onclick={addTodo}>{$t.todo.addTask}</button>
  </div>

  {#if view === "board"}
    <div class="kanban">
      {#each columns as c}
        <div class="kanban-col">
          <div class="col-head">{colTitle(c)} <span class="count">{colTodos(c).length}</span></div>
          <div class="col-body">
            {#each colTodos(c) as td}
              <div class="todo-card" class:due={isDueSoon(td)}>
                <div class="card-top">
                  <span class="prio prio-{td.priority}">{$t.todo.priority[td.priority]}</span>
                  {#if td.tag}<span class="tag">#{td.tag}</span>{/if}
                </div>
                <div class="card-title">{td.title}</div>
                {#if td.due_date}
                  <div class="card-due">📅 {td.due_date}</div>
                {/if}
                {#if td.items && td.items.length}
                  <div class="subtasks">
                    {#each td.items as it}
                      <label class="sub-item">
                        <input type="checkbox" checked={!!it.done} onchange={() => toggleSubtask(it.id, !it.done)} />
                        <span class:done={!!it.done}>{it.label}</span>
                      </label>
                    {/each}
                  </div>
                {/if}
                <div class="card-actions">
                  {#if c === "todo"}
                    <button class="mini" onclick={() => updateStatus(td.id, "doing")}>→ {$t.todo.doing}</button>
                  {/if}
                  {#if c === "doing"}
                    <button class="mini" onclick={() => updateStatus(td.id, "done")}>{$t.todo.complete}</button>
                  {/if}
                  {#if c === "done"}
                    <button class="mini" onclick={() => updateStatus(td.id, "todo")}>↩</button>
                  {/if}
                  <button class="mini danger" onclick={() => deleteTodo(td.id)}>{$t.todo.delete}</button>
                </div>
              </div>
            {:else}
              <div class="col-empty">—</div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if view === "list" || view === "all"}
    <div class="list-view">
      {#each $todos as td}
        <div class="list-item">
          <input type="checkbox" checked={td.status === "done"} onchange={() => updateStatus(td.id, td.status === "done" ? "todo" : "done")} />
          <span class:line={td.status === "done"}>{td.title}</span>
          <span class="prio prio-{td.priority}">{$t.todo.priority[td.priority]}</span>
          {#if td.tag}<span class="tag">#{td.tag}</span>{/if}
        </div>
      {:else}
        <div class="empty">{$t.todo.empty}</div>
      {/each}
    </div>
  {:else if view === "today"}
    <div class="list-view">
      {#each $todos.filter((td) => td.due_date === new Date().toISOString().slice(0, 10)) as td}
        <div class="list-item">
          <input type="checkbox" checked={td.status === "done"} onchange={() => updateStatus(td.id, td.status === "done" ? "todo" : "done")} />
          <span class:line={td.status === "done"}>{td.title}</span>
        </div>
      {:else}
        <div class="empty">{$t.todo.empty}</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .todo-view {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
  }
  .todo-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
  }
  .todo-header h2 {
    font-size: 20px;
  }
  .view-tabs {
    display: flex;
    gap: 4px;
  }
  .tab {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    font-size: 13px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
  }
  .tab.active {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent-dim);
  }
  .todo-add {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }
  .todo-add input:not([type]) {
    flex: 1;
  }
  .kanban {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    align-items: start;
  }
  .kanban-col {
    background: var(--bg-panel);
    border: 1px solid var(--border-soft);
    border-radius: var(--radius);
    padding: 10px;
    min-height: 200px;
  }
  .col-head {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    margin-bottom: 8px;
    display: flex;
    justify-content: space-between;
  }
  .count {
    background: var(--bg-elev);
    border-radius: 8px;
    padding: 1px 7px;
    font-size: 11px;
  }
  .col-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .todo-card {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px;
  }
  .todo-card.due {
    border-color: var(--warn);
  }
  .card-top {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: 6px;
  }
  .prio {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 600;
  }
  .prio-high {
    background: rgba(255, 143, 122, 0.15);
    color: var(--danger);
  }
  .prio-medium {
    background: rgba(255, 210, 110, 0.15);
    color: var(--warn);
  }
  .prio-low {
    background: rgba(122, 233, 152, 0.12);
    color: var(--accent);
  }
  .tag {
    font-size: 10px;
    color: var(--blue);
  }
  .card-title {
    font-size: 13.5px;
    font-weight: 500;
    margin-bottom: 6px;
  }
  .card-due {
    font-size: 11px;
    color: var(--text-faint);
    margin-bottom: 4px;
  }
  .subtasks {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 6px 0;
  }
  .sub-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .sub-item .done {
    text-decoration: line-through;
    color: var(--text-faint);
  }
  .card-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .mini {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-dim);
  }
  .mini:hover {
    color: var(--text);
    border-color: var(--text-faint);
  }
  .mini.danger:hover {
    color: var(--danger);
    border-color: var(--danger);
  }
  .col-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    padding: 20px 0;
  }
  .list-view {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .list-item {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-panel);
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
  }
  .list-item .line {
    text-decoration: line-through;
    color: var(--text-faint);
  }
  .empty {
    color: var(--text-faint);
    text-align: center;
    padding: 40px;
  }
</style>
