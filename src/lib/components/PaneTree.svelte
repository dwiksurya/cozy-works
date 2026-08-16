<script>
  // Recursive layout renderer for the multiplexer tree.
  // node: { type:"leaf", paneId } | { type:"branch", dir:"row"|"col", ratio, children:[node] }
  export let tab;
  export let startDrag;
  export let containerAction;
  export let focusPane;
  export let node = null; // optional: subtree root (defaults to tab.root)
  $: root = node || tab.root;
</script>

{#if root.type === "leaf"}
  <div
    class="pane-wrap {tab.activePane === root.paneId ? 'active' : ''}"
    style="flex:1;min-width:0;min-height:0;display:flex;flex-direction:column;"
    onclick={() => focusPane(root.paneId)}
  >
    <div class="pane-titlebar"><span class="pane-title">{root.paneId}</span></div>
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
    padding: 2px 6px;
    font-family: var(--font-body);
    font-size: 11px;
    color: var(--text-faint);
    background: var(--surface-container-low);
    border-bottom: 1px solid var(--border);
    user-select: none;
    flex-shrink: 0;
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }
</style>
