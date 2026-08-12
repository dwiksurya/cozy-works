<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Terminal } from "xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "xterm/css/xterm.css";
  import { t } from "../i18n-store.js";
  import { toast } from "../stores.js";

  let containerEl;
  let term;
  let fitAddon;
  let termId = null;
  let unlisten = [];
  let status = { dir: "", branch: null, dirty: false };
  let statusTimer;
  let promptText = "";
  let promptTimer;

  const history = [];
  let historyPos = -1;

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  async function init() {
    if (!isTauri()) {
      // browser fallback: fake terminal
      term = new Terminal();
      fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.loadAddon(new WebLinksAddon());
      term.open(containerEl);
      term.writeln("\x1b[36mCozy Works terminal — running in browser (demo mode)\x1b[0m");
      term.writeln("(Tauri PTY not available here)");
      term.onData((d) => {
        term.write(d);
        if (d === "\r") term.write("\r\n$ ");
      });
      fitAddon.fit();
      return;
    }

    term = new Terminal({
      fontSize: 13,
      fontFamily: "JetBrains Mono, ui-monospace, monospace",
      cursorBlink: true,
      theme: {
        background: "#12151f",
        foreground: "#e8e6df",
        cursor: "#7ae998",
        selectionBackground: "#2c3246",
        black: "#12151f",
        red: "#ff8f7a",
        green: "#7ae998",
        yellow: "#ffd26e",
        blue: "#8ea8ff",
        magenta: "#b98eff",
        cyan: "#7adbe9",
        white: "#e8e6df",
        brightBlack: "#6b7488",
        brightRed: "#ff8f7a",
        brightGreen: "#7ae998",
        brightYellow: "#ffd26e",
        brightBlue: "#8ea8ff",
        brightMagenta: "#b98eff",
        brightCyan: "#7adbe9",
        brightWhite: "#ffffff",
      },
      scrollback: 5000,
    });
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.open(containerEl);

    // listen PTY events
    unlisten.push(
      await listen("pty://output", (e) => {
        const { id, data } = e.payload;
        if (id === termId) {
          term.write(data);
          parsePrompt(data);
        }
      })
    );
    unlisten.push(
      await listen("pty://exit", (e) => {
        if (e.payload.id === termId) {
          term.writeln("\r\n\x1b[33m[process exited]\x1b[0m");
          termId = null;
          refreshStatus();
        }
      })
    );

    await newTerminal();
    term.onData((data) => {
      if (!termId) return;
      invoke("write_terminal", { id: termId, data });
      if (data === "\r") historyPos = -1;
    });
    // track history
    term.attachCustomKeyEventHandler((e) => {
      if (e.key === "ArrowUp" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        return true; // allow default
      }
      return false;
    });

    fitAddon.fit();
    window.addEventListener("resize", () => fitAddon.fit());
  }

  async function newTerminal() {
    if (!isTauri()) return;
    try {
      const info = await invoke("spawn_terminal");
      termId = info.id;
      term.reset();
      refreshStatus();
    } catch (e) {
      toast(`Terminal error: ${e}`, "error");
    }
  }

  function refreshStatus() {
    if (!termId || !isTauri()) {
      status = { dir: "", branch: null, dirty: false };
      return;
    }
    // get cwd from PTY is complex; approximate with HOME for now, backend will
    // improve later. For v1 we track cwd via shell prompt parsing.
    if (promptText) {
      const m = promptText.match(/(~?\/[^\s$]*)/);
      status.dir = m ? m[1] : "";
    } else {
      status.dir = "~";
    }
    invoke("git_branch", { cwd: status.dir.replace("~", homeDir()) })
      .then((r) => {
        status = { dir: status.dir, branch: r.branch, dirty: r.dirty };
      })
      .catch(() => {});
  }

  function homeDir() {
    return window.__TAURI_INTERNALS__ ? "/home/ubuntu" : "/home/ubuntu";
  }

  function parsePrompt(data) {
    // detect cwd from prompt like: user@host:/path$
    const lines = data.split("\n");
    const last = lines[lines.length - 1];
    const m = last.match(/([^\s]*\/[^\s$]*)\s*[\$#]/);
    if (m) {
      promptText = m[1];
      clearTimeout(promptTimer);
      promptTimer = setTimeout(refreshStatus, 400);
    }
  }

  onMount(async () => {
    await init();
  });

  onDestroy(() => {
    unlisten.forEach((u) => u());
    if (termId && isTauri()) invoke("kill_terminal", { id: termId });
    term?.dispose();
    window.removeEventListener("resize", () => fitAddon?.fit());
  });
</script>

<div class="terminal-view">
  <div class="term-statusbar">
    <span class="st-item">⏻ {status.dir || "~"}</span>
    {#if status.branch}
      <span class="st-item branch">
        <span class="branch-icon">⎇</span> {status.branch}
        {#if status.dirty}<span class="dirty">●</span>{/if}
      </span>
    {/if}
    <span class="st-spacer"></span>
    <button class="st-btn" onclick={newTerminal} title={$t.terminal.newTab}>+</button>
    <button class="st-btn" onclick={() => invoke("kill_terminal", { id: termId })} title={$t.terminal.closeTab}>✕</button>
  </div>
  <div class="term-container" bind:this={containerEl}></div>
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #12151f;
  }
  .term-statusbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border-soft);
    font-size: 11.5px;
    color: var(--text-dim);
    font-family: var(--font-mono);
  }
  .st-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: var(--bg-elev);
    border-radius: 4px;
  }
  .st-item.branch {
    color: var(--accent);
  }
  .branch-icon {
    font-size: 12px;
  }
  .dirty {
    color: var(--warn);
    font-size: 8px;
  }
  .st-spacer {
    flex: 1;
  }
  .st-btn {
    font-size: 13px;
    color: var(--text-dim);
    padding: 1px 7px;
    border-radius: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
  }
  .st-btn:hover {
    color: var(--text);
  }
  .term-container {
    flex: 1;
    overflow: hidden;
    padding: 6px 0 0 6px;
  }
  :global(.xterm) {
    height: 100%;
  }
</style>
