<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Terminal } from "xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "xterm/css/xterm.css";
  import { t } from "../i18n-store.js";
  import { toast, workspaceStatus, terminalTabs, panes } from "../stores.js";
  import { get } from "svelte/store";
  import Icon from "./Icon.svelte";
  import PaneTree from "./PaneTree.svelte";

  // ============================================================
  // Layout model — multiplexer tree (tmux/herdr style)
  //   Node = { type: "leaf", paneId } | { type: "branch", dir: "row"|"col", ratio, children: [Node] }
  //   dir "row" = horizontal split (children side by side)
  //   dir "col" = vertical split (children stacked)
  // ============================================================
  let tabs = []; // [{id, label, labelFull, root: Node, activePane: paneId, termById: {} }]
  let activeTabId = null;
  let containerRefs = {}; // paneId -> DOM element
  let unlisten = [];
  let loading = true;

  // pane counter (unique ids; terminal ids come from backend)
  let paneSeq = 1;
  const nextPaneId = () => `p${paneSeq++}`;

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function leaf(paneId) {
    return { type: "leaf", paneId };
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

  // ---- layout helpers ----
  function collectPaneIds(node, out = []) {
    if (!node) return out;
    if (node.type === "leaf") out.push(node.paneId);
    else node.children.forEach((c) => collectPaneIds(c, out));
    return out;
  }

  function paneCount(node) {
    if (!node) return 0;
    return node.type === "leaf" ? 1 : node.children.reduce((a, c) => a + paneCount(c), 0);
  }

  function findPaneNode(node, paneId) {
    if (!node) return null;
    if (node.type === "leaf") return node.paneId === paneId ? node : null;
    for (const c of node.children) {
      const r = findPaneNode(c, paneId);
      if (r) return r;
    }
    return null;
  }

  function firstPaneId(node) {
    if (!node) return null;
    if (node.type === "leaf") return node.paneId;
    return firstPaneId(node.children[0]);
  }

  // Split the node containing paneId; if its parent is already a branch in
  // the same direction, append a sibling (tmux-style) instead of nesting.
  function splitPane(root, paneId, dir) {
    const parent = findParentNode(root, paneId);
    if (parent && parent.type === "branch" && parent.dir === dir) {
      // append sibling to existing same-dir branch
      const newPane = nextPaneId();
      parent.children.push(leaf(newPane));
      parent.ratio = 1 / parent.children.length;
      return newPane;
    }
    // replace leaf with branch
    const newPane = nextPaneId();
    const branch = {
      type: "branch",
      dir,
      ratio: 0.5,
      children: [leaf(paneId), leaf(newPane)],
    };
    if (!parent) {
      // root replaced
      Object.assign(root, branch);
    } else {
      const idx = parent.children.findIndex((c) => c.type === "leaf" && c.paneId === paneId);
      if (idx !== -1) parent.children[idx] = branch;
    }
    return newPane;
  }

  // Find the parent of a node. `target` may be a paneId string (match leaf by
  // id) or a branch node reference (match by identity, used during collapse).
  function findParentNode(node, target) {
    if (!node || node.type === "leaf") return null;
    for (const c of node.children) {
      if (c === target) return node;
      if (c.type === "leaf" && c.paneId === target) return node;
      const r = findParentNode(c, target);
      if (r) return r;
    }
    return null;
  }

  // Remove a leaf. Collapse single-child branches upward.
  function closePane(root, paneId) {
    const parent = findParentNode(root, paneId);
    if (!parent) return false; // last pane — cannot close
    const idx = parent.children.findIndex((c) => c.type === "leaf" && c.paneId === paneId);
    if (idx === -1) return false;
    parent.children.splice(idx, 1);
    if (parent.children.length === 1) {
      // collapse branch into its single child
      const only = parent.children[0];
      const grandparent = findParentNode(root, parent);
      if (!grandparent) {
        // root is the branch — replace contents with the only child
        Object.keys(root).forEach((k) => delete root[k]);
        Object.assign(root, only);
      } else {
        const gi = grandparent.children.findIndex((c) => c === parent);
        grandparent.children[gi] = only;
      }
    } else {
      parent.ratio = 1 / parent.children.length;
    }
    return true;
  }

  // ---- tab lifecycle ----
  async function newTab() {
    if (!isTauri()) return;
    try {
      loading = true;
      const info = await invoke("spawn_terminal");
      loading = false;
      const paneId = nextPaneId();
      const tab = {
        id: info.id, // tab id == first pane backend id (back-compat)
        label: "shell",
        labelFull: "",
        root: leaf(paneId),
        activePane: paneId,
        terms: {}, // paneId -> {term, fit, mounted}
        paneIds: new Set([paneId]),
        termBackend: { [paneId]: info.id }, // paneId -> backend terminal id
      };
      tabs = [...tabs, tab];
      activeTabId = tab.id;
      syncTabs();
      setTimeout(() => {
        const tb = tabs.find((x) => x.id === tab.id);
        if (tb) {
          tb.label = "~";
          tb.labelFull = "~";
        }
        syncTabs();
      }, 300);
      return tab.id;
    } catch (e) {
      toast(`Terminal error: ${e}`, "error");
      return null;
    }
  }

  function closeTab(id) {
    const idx = tabs.findIndex((x) => x.id === id);
    if (idx === -1) return;
    const tab = tabs[idx];
    tab.terms.forEach(({ term }) => term?.dispose());
    // kill all backend terminals in this tab
    Object.values(tab.termBackend || {}).forEach((bid) => invoke("kill_terminal", { id: bid }).catch(() => {}));
    tabs = tabs.filter((x) => x.id !== id);
    if (activeTabId === id) {
      activeTabId = tabs.length ? tabs[tabs.length - 1].id : null;
    }
    syncTabs();
    if (!tabs.length) refreshWorkspace();
  }

  // ---- split / close pane from UI ----
  async function doSplit(dir) {
    const tab = tabs.find((x) => x.id === activeTabId);
    if (!tab || !isTauri()) return;
    const info = await invoke("spawn_terminal");
    const paneId = splitPane(tab.root, tab.activePane, dir);
    tab.paneIds.add(paneId);
    // map pane -> backend terminal id
    tab.termBackend = tab.termBackend || {};
    tab.termBackend[paneId] = info.id;
    tab.activePane = paneId;
    syncTabs();
    setTimeout(() => {
      const t = tab.terms[paneId];
      t?.fit?.fit();
      t?.term?.focus();
    }, 60);
  }

  async function doClosePane() {
    const tab = tabs.find((x) => x.id === activeTabId);
    if (!tab) return;
    const removed = tab.activePane;
    const backendId = tab.termBackend?.[removed];
    const ok = closePane(tab.root, removed);
    if (ok) {
      tab.paneIds.delete(removed);
      tab.terms[removed]?.term?.dispose();
      delete tab.terms[removed];
      delete tab.termBackend?.[removed];
      if (backendId != null) invoke("kill_terminal", { id: backendId }).catch(() => {});
      tab.activePane = firstPaneId(tab.root);
      syncTabs();
    }
  }

  function focusPane(paneId) {
    const tab = tabs.find((x) => x.id === activeTabId);
    if (!tab) return;
    tab.activePane = paneId;
    syncTabs();
    setTimeout(() => {
      const t = tab.terms[paneId];
      t?.fit?.fit();
      t?.term?.focus();
    }, 40);
  }

  function zoomPane() {
    const tab = tabs.find((x) => x.id === activeTabId);
    if (!tab) return;
    tab.zoom = tab.zoom === tab.activePane ? null : tab.activePane;
    syncTabs();
  }

  // ---- rendering ----
  function syncTabs() {
    terminalTabs.set(
      tabs.map((x) => ({ id: x.id, label: x.label, labelFull: x.labelFull, active: x.id === activeTabId }))
    );
    if (typeof window !== "undefined") window.__activeTerminalId = activeTabId;
    setTimeout(mountTerms, 20);
  }

  // All pane ids that should be mounted (visible or zoomed)
  function visiblePaneIds(tab) {
    if (!tab) return [];
    if (tab.zoom) return [tab.zoom];
    return collectPaneIds(tab.root);
  }

  function mountTerms() {
    for (const tab of tabs) {
      const visible = new Set(visiblePaneIds(tab));
      // mount visible
      for (const pid of visible) {
        const el = containerRefs[`${tab.id}:${pid}`];
        if (!el) continue;
        const backendId = tab.termBackend?.[pid] ?? (pid === firstPaneId(tab.root) ? tab.id : null);
        if (backendId == null) continue;
        let entry = tab.terms[pid];
        if (!el.dataset.mounted && !entry) {
          const { term, fit } = createTerm();
          term.open(el);
          term.writeln("\x1b[36m~ cozy-works terminal\x1b[0m");
          entry = { term, fit, backendId };
          tab.terms[pid] = entry;
          fit.fit();
          term.onData((data) => {
            if (tab.activePane !== pid) return;
            invoke("write_terminal", { id: backendId, data }).catch(() => {});
            if (data === "\r") parseOutput(tab, data);
          });
          el.dataset.mounted = "1";
          if (tab.activePane === pid) setTimeout(() => term.focus(), 30);
        } else if (entry) {
          entry.backendId = backendId;
        }
        el.style.display = "block";
        if (tab.activePane === pid) {
          setTimeout(() => {
            entry?.fit?.fit();
            entry?.term?.focus();
          }, 50);
        }
      }
      // hide non-visible
      for (const pid of tab.paneIds) {
        const el = containerRefs[`${tab.id}:${pid}`];
        if (el && !visible.has(pid)) el.style.display = "none";
      }
    }
  }

  function containerAction(node, key) {
    containerRefs[key] = node;
    return {
      destroy() {
        delete containerRefs[key];
      },
    };
  }

  function parseOutput(tab, data) {
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

  // ---- PTY events ----
  async function initListeners() {
    unlisten.push(
      await listen("pty://output", (e) => {
        const { id, data } = e.payload;
        // find tab/pane that maps to this backend terminal id
        for (const tab of tabs) {
          const pid = Object.keys(tab.termBackend || {}).find((k) => tab.termBackend[k] === id);
          const entry = tab.terms[pid];
          if (entry) {
            entry.term.write(data);
            if (tab.activePane === pid) parseOutput(tab, data);
          }
        }
        if (loading) loading = false;
      })
    );
    unlisten.push(
      await listen("pty://exit", (e) => {
        const { id } = e.payload;
        for (const tab of tabs) {
          const pid = Object.keys(tab.termBackend || {}).find((k) => tab.termBackend[k] === id);
          const entry = tab.terms[pid];
          if (entry) entry.term.writeln("\r\n\x1b[33m[process exited]\x1b[0m");
        }
      })
    );
  }

  // ---- pane info polling (process name + status for titlebars) ----
  let panesTimer = null;
  async function pollPanes() {
    if (!isTauri()) return;
    try {
      const list = await window.__TAURI_INTERNALS__.invoke("list_panes");
      panes.set(list);
    } catch (e) {
      /* backend not ready */
    }
  }

  // ---- keyboard shortcuts (tmux-style Ctrl+B prefix) ----
  let prefixActive = false;
  let prefixTimer = null;

  function onKeydown(e) {
    // if typing inside xterm, xterm handles keys — but we intercept global
    // shortcuts only when NOT in an editable field (xterm textarea is .xterm-helper-textarea)
    if (prefixActive) {
      e.preventDefault();
      const tab = tabs.find((x) => x.id === activeTabId);
      const k = e.key.toLowerCase();
      if (k === "%" || (e.shiftKey && k === "5")) doSplit("row"); // Ctrl+B % → horizontal split
      else if (k === '"' || (e.shiftKey && k === "'")) doSplit("col"); // Ctrl+B " → vertical split
      else if (k === "arrowleft") moveFocus(-1, 0);
      else if (k === "arrowright") moveFocus(1, 0);
      else if (k === "arrowup") moveFocus(0, -1);
      else if (k === "arrowdown") moveFocus(0, 1);
      else if (k === "x") doClosePane();
      else if (k === "z") zoomPane();
      else if (k === "c") newTab();
      else if (k === "escape" || k === "q") { /* exit prefix */ }
      clearPrefixTimer();
      prefixActive = false;
      return;
    }
    if (e.ctrlKey && e.key.toLowerCase() === "b") {
      e.preventDefault();
      prefixActive = true;
      clearPrefixTimer();
      prefixTimer = setTimeout(() => (prefixActive = false), 1500);
    }
  }

  function clearPrefixTimer() {
    if (prefixTimer) clearTimeout(prefixTimer);
  }

  function moveFocus(dx, dy) {
    const tab = tabs.find((x) => x.id === activeTabId);
    if (!tab) return;
    const ids = collectPaneIds(tab.root);
    if (ids.length < 2) return;
    const idx = ids.indexOf(tab.activePane);
    if (idx === -1) return;
    let next = idx;
    if (dx !== 0) {
      // simple ordering: next in array (right) / prev (left)
      next = (idx + dx + ids.length) % ids.length;
    } else if (dy !== 0) {
      next = (idx + dy + ids.length) % ids.length;
    }
    focusPane(ids[next]);
  }

  // ---- drag resize ----
  function startDrag(ev, dir, tabId, branch) {
    ev.preventDefault();
    const tab = tabs.find((x) => x.id === tabId);
    if (!tab) return;
    const startX = ev.clientX;
    const startY = ev.clientY;
    const startRatio = branch.ratio;
    const container = ev.currentTarget.parentElement;
    const rect = container.getBoundingClientRect();
    function onMove(mv) {
      let delta = dir === "row" ? mv.clientX - startX : mv.clientY - startY;
      let ratio = startRatio + (dir === "row" ? delta / rect.width : delta / rect.height);
      ratio = Math.min(0.85, Math.max(0.15, ratio));
      branch.ratio = ratio;
      syncTabs();
    }
    function onUp() {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  onMount(async () => {
    if (isTauri()) {
      await initListeners();
      await newTab();
      pollPanes();
      panesTimer = setInterval(pollPanes, 2000);
    } else {
      const id = "browser";
      const paneId = nextPaneId();
      tabs = [{
        id,
        label: "demo",
        labelFull: "",
        root: leaf(paneId),
        activePane: paneId,
        terms: {},
        paneIds: new Set([paneId]),
        termBackend: {},
      }];
      activeTabId = id;
      loading = false;
      syncTabs();
      setTimeout(() => {
        const tab = tabs[0];
        const entry = tab.terms[paneId];
        if (entry && entry.term) {
          entry.term.writeln("\x1b[36mCozy Works terminal — browser demo mode\x1b[0m");
          entry.term.onData((d) => {
            entry.term.write(d);
            if (d === "\r") entry.term.write("\r\n$ ");
          });
        }
      }, 100);
    }
    window.__focusTerminalTab = (termId) => {
      // termId is the backend terminal id of the pane where the agent runs.
      // Find the tab+pane that maps to it.
      for (const tab of tabs) {
        for (const [pid, bid] of Object.entries(tab.termBackend || {})) {
          if (bid === termId) {
            activeTabId = tab.id;
            tab.activePane = pid;
            syncTabs();
            setTimeout(() => {
              const entry = tab.terms[pid];
              entry?.fit?.fit();
              entry?.term?.focus();
            }, 40);
            return;
          }
        }
      }
    };
    window.__focusPane = (paneId) => focusPane(paneId);
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("resize", () => {
      const active = tabs.find((x) => x.id === activeTabId);
      if (!active) return;
      collectPaneIds(active.root).forEach((pid) => {
        active.terms[pid]?.fit?.fit();
      });
    });
  });

  onDestroy(() => {
    unlisten.forEach((u) => u());
    if (panesTimer) clearInterval(panesTimer);
    window.removeEventListener("keydown", onKeydown);
    for (const tab of tabs) {
      tab.terms.forEach(({ term }) => term?.dispose());
      const ids = collectPaneIds(tab.root);
      ids.forEach((pid) => {
        const backendId = tab.termBackend?.[pid];
        if (backendId != null) invoke("kill_terminal", { id: backendId }).catch(() => {});
      });
    }
  });
</script>

<div class="terminal-view">
  <div class="tab-bar">
    {#each tabs as tab}
      <div class="term-tab" class:active={tab.id === activeTabId} onclick={() => { activeTabId = tab.id; syncTabs(); }} title={tab.labelFull || tab.label}>
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
    <span class="tab-hint">ctrl+b % split · " vsplit · ←→↑↓ nav · x close · z zoom</span>
  </div>

  <div class="term-container">
    {#if loading}
      <div class="term-loading">
        <span class="pixel-spinner"></span>
        <span>Starting shell…</span>
      </div>
    {/if}

    {#each tabs as tab}
      {#if tab.id === activeTabId}
        <div class="term-mux">
          {#if tab.zoom}
            <div class="pane-wrap active" style="flex:1;min-width:0;min-height:0;display:flex;flex-direction:column;">
              <div class="pane-titlebar">
                <span class="pane-name">zoom: {tab.zoom} <small>(ctrl+b z to unzoom)</small></span>
              </div>
              <div class="pane-body" use:containerAction={`${tab.id}:${tab.zoom}`}></div>
            </div>
          {:else}
            <PaneTree {tab} {startDrag} {containerAction} {focusPane} />
          {/if}
        </div>
      {/if}
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
    font-size: 11px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    pointer-events: none;
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
    to { transform: rotate(360deg); }
  }
  .term-mux {
    position: absolute;
    inset: 0;
    display: flex;
    padding: 2px;
    gap: 0;
  }
  .branch {
    display: flex;
    width: 100%;
    height: 100%;
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
  :global(.xterm) {
    height: 100%;
  }
</style>
