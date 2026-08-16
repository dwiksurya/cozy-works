<script>
  // Recursive layout renderer for the multiplexer tree.
  // node: { type:"leaf", paneId } | { type:"branch", dir:"row"|"col", ratio, children:[node] }
  import { panes } from "../stores.js";
  export let tab;
  export let startDrag;
  export let containerAction;
  export let focusPane;
  export let node = null; // optional: subtree root (defaults to tab.root)
  $: root = node || tab.root;

  // Look up pane info (process name + status + osc title) by backend terminal id.
  function paneInfo(paneId) {
    const bid = tab.termBackend?.[paneId];
    if (bid == null) return null;
    return $panes.find((p) => p.terminal_id === bid) || null;
  }

  // status icon for pane titlebar
  function statusIcon(status) {
    return status === "blocker" ? "⚠" : status === "running" ? "⏳" : "✓";
  }

  // strip leading status chars from OSC title
  const SPINNER_CHARS = new Set(["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏","⠁","⠂","⠄","⡀","⢀","⣀"]);
  function stripStatusIcon(title) {
    if (!title) return "";
    let s = String(title).trim();
    while (s.length) {
      const c = s[0];
      if (SPINNER_CHARS.has(c) || c === "⏳" || c === "⚠" || c === "✓" || c === "✳" || c === "⌛") {
        s = s.slice(1).trimStart();
      } else break;
    }
    return s;
  }

  function paneTitle(paneId) {
    const info = paneInfo(paneId);
    if (!info) return "shell";
    let t = info.name;
    if (info.title) {
      const clean = stripStatusIcon(info.title);
      if (clean) t += ` · ${clean}`;
    }
    return t;
  }
</script>

{#if root.type === "leaf"}
  <div
    class="pane-wrap {tab.activePane === root.paneId ? 'active' : ''}"
    style="flex:1;min-width:0;min-height:0;display:flex;flex-direction:column;"
    onclick={() => focusPane(root.paneId)}
  >
    <div class="pane-titlebar" title={paneTitle(root.paneId)}>
      {#if paneInfo(root.paneId)}
        <span class="pane-dot" class:running={paneInfo(root.paneId).status === "running"} class:blocker={paneInfo(root.paneId).status === "blocker"} class:idle={paneInfo(root.paneId).status === "idle"}></span>
      {/if}
      <span class="pane-name">{paneTitle(root.paneId)}</span>
    </div>
    <div class="pane-body" use:containerAction={`${tab.id}:${root.paneId}`}></div>
  </div>
{:else}
  <div
    class="branch {root.dir}"
    style="display:flex;flex-direction:{root.dir === 'row' ? 'row' : 'column'};width:100%;height:100%;"
  >
    {#each root.children as child, i}
      <div
        class="branch-child"
        style="flex:{root.ratio};min-width:0;min-height:0;display:flex;flex-direction:{root.dir === 'row' ? 'row' : 'column'};"
      >
        <PaneTree {tab} {startDrag} {containerAction} {focusPane} node={child} />
      </div>
      {#if i < root.children.length - 1}
        <div class="pane-divider {root.dir}" onmousedown={(e) => startDrag(e, root.dir, tab.id, root)}></div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .branch {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }
  .branch-child {
    display: flex;
    min-width: 0;
    min-height: 0;
    position: relative;
  }
  .pane-divider {
    flex: 0 0 4px;
    background: var(--border);
    cursor: col-resize;
    position: relative;
    z-index: 5;
  }
  .pane-divider.col {
    cursor: row-resize;
  }
  .pane-divider:hover {
    background: var(--primary);
  }
  .pane-wrap {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border);
    background: var(--surface);
    flex: 1;
  }
  .pane-wrap.active {
    border-color: var(--primary);
  }
  .pane-titlebar {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 2px 6px;
    font-family: var(--font-body);
    font-size: 11px;
    color: var(--text-faint);
    background: var(--surface-container-low);
    border-bottom: 1px solid var(--border);
    user-select: none;
    flex-shrink: 0;
    overflow: hidden;
  }
  .pane-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pane-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
    flex-shrink: 0;
  }
  .pane-dot.running {
    background: var(--checklist-green);
    animation: pane-dot-blink 1s steps(2, start) infinite;
  }
  .pane-dot.blocker {
    background: var(--danger);
  }
  .pane-dot.idle {
    background: var(--accent);
    opacity: 0.7;
  }
  @keyframes pane-dot-blink {
    0%, 49% { opacity: 1; }
    50%, 100% { opacity: 0.35; }
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }
</style>
