<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { settings, toast, activeView, showAiSidebar } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { get } from "svelte/store";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

  let input = "";
  let messages = [];
  let thinking = false;
  let streamAbort = null;
  let agentMode = false; // agent mode: can propose confirmations

  const MODELS = [
    { id: "ds/deepseek-v4-flash", label: "DeepSeek V4 Flash (fast)" },
    { id: "ds/deepseek-v4-pro", label: "DeepSeek V4 Pro" },
    { id: "ds/deepseek-reasoner", label: "DeepSeek Reasoner" },
    { id: "cmc/deepseek/deepseek-v4-flash", label: "DeepSeek V4 Flash (cmc)" },
    { id: "cmc/deepseek/deepseek-v4-pro", label: "DeepSeek V4 Pro (cmc)" },
    { id: "ag/claude-sonnet-4-6", label: "Claude Sonnet 4.6" },
    { id: "ag/claude-opus-4-6-thinking", label: "Claude Opus 4.6" },
    { id: "ag/gemini-3.5-flash", label: "Gemini 3.5 Flash" },
    { id: "cmc/Qwen/Qwen3.6-Plus", label: "Qwen 3.6 Plus" },
  ];

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  async function getRouterKey() {
    // Try to read from Hermes env via a lightweight probe — no secret exposure.
    // If Hermes gateway is running, we can reuse its credential pool through
    // the local proxy (127.0.0.1:1430) without knowing the key.
    try {
      const resp = await fetch("http://127.0.0.1:1430/v1/models", {
        method: "GET",
        headers: { Authorization: "Bearer local" },
        signal: AbortSignal.timeout(1500),
      });
      if (resp.ok) return "local"; // Hermes proxy accepts any token
    } catch (e) {
      /* no proxy — fall through */
    }
    return "";
  }

  async function ensureNotif() {
    if (!isTauri()) return false;
    try {
      let granted = await isPermissionGranted();
      if (!granted) granted = (await requestPermission()) === "granted";
      return granted;
    } catch {
      return false;
    }
  }

  function notify(title, body) {
    if (!isTauri()) return;
    ensureNotif().then((ok) => {
      if (ok) sendNotification({ title, body });
    });
  }

  function modelLabel(id) {
    const m = MODELS.find((x) => x.id === id);
    return m ? m.label : id;
  }

  async function send() {
    const text = input.trim();
    if (!text || thinking) return;
    input = "";
    messages.push({ role: "user", content: text });
    scrollBottom();
    thinking = true;

    const $s = get(settings);
    const baseUrl = $s.aiBaseUrl || "https://router.takora.dev/v1";

    // fetch api key from settings — stored via setApiKey (not hardcoded)
    // Fallback: read from Hermes config (providers.9router.api_key) via a small bridge
    const apiKey = $s.aiApiKey || (await getRouterKey()) || "";

    // push placeholder assistant message for streaming
    const msgIdx = messages.push({ role: "assistant", content: "" }) - 1;

    try {
      const resp = await fetch(`${baseUrl}/chat/completions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          model: $s.aiModel || "ds/deepseek-v4-flash",
          messages: [
            {
              role: "system",
              content:
                "You are the AI assistant inside Cozy Works, a cozy pixel-zen workspace app. " +
                "Answer concisely and helpfully. When suggesting shell commands, put them in code blocks. " +
                "If you need user confirmation before running something, say 'NEEDS_CONFIRM:' followed by the command.",
            },
            ...messages.filter((m) => m.content),
          ],
          stream: true,
          temperature: 0.7,
        }),
      });

      if (!resp.ok) {
        const errText = await resp.text();
        throw new Error(`API error ${resp.status}: ${errText.slice(0, 200)}`);
      }

      const reader = resp.body.getReader();
      const decoder = new TextDecoder();
      let acc = "";
      streamAbort = new AbortController();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        acc += decoder.decode(value, { stream: true });
        // parse SSE lines
        const lines = acc.split("\n");
        acc = lines.pop();
        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed.startsWith("data:")) continue;
          const payload = trimmed.slice(5).trim();
          if (payload === "[DONE]") continue;
          try {
            const j = JSON.parse(payload);
            const delta = j.choices?.[0]?.delta?.content || "";
            if (delta) {
              messages[msgIdx].content += delta;
              scrollBottom();
            }
          } catch {
            /* partial json */
          }
        }
      }

      // notify on completion
      if ($s.aiNotifyOnDone) {
        notify("Cozy Works AI", `AI finished replying: ${text.slice(0, 40)}…`);
      }

      // detect confirmation request
      const final = messages[msgIdx].content;
      const confirmMatch = final.match(/NEEDS_CONFIRM:\s*([^\n]+)/);
      if (confirmMatch && $s.aiNotifyOnAsk) {
        notify("Cozy Works AI", `AI needs confirmation: ${confirmMatch[1]}`);
        // store pending command
        pendingCommand = confirmMatch[1];
        pendingVisible = true;
      }
    } catch (e) {
      messages[msgIdx].content = `\n\n[Error: ${e.message}]`;
    } finally {
      thinking = false;
    }
  }

  // pending command confirmation UI
  let pendingCommand = "";
  let pendingVisible = false;

  function confirmCommand() {
    if (!pendingCommand) return;
    // insert command into terminal
    insertToTerminal(pendingCommand);
    pendingVisible = false;
  }

  function dismissConfirm() {
    pendingVisible = false;
  }

  function insertToTerminal(cmd) {
    // send command to terminal: write to PTY + enter
    const clean = cmd.trim().replace(/^```[a-z]*\n?/, "").replace(/```$/, "").trim();
    invoke("write_terminal", { id: getActiveTerminalId(), data: clean + "\r" })
      .then(() => {
        toast("Command inserted into terminal", "info");
      })
      .catch((e) => {
        toast(`Insert failed: ${e}`, "error");
      });
  }

  function getActiveTerminalId() {
    // global terminal registry — set by Terminal component
    return window.__activeTerminalId || 0;
  }

  function scrollBottom() {
    setTimeout(() => {
      const el = document.querySelector(".ai-messages");
      if (el) el.scrollTop = el.scrollHeight;
    }, 50);
  }

  function clearChat() {
    messages = [];
  }

  function parseCodeBlocks(content) {
    // find code blocks for insert buttons
    const blocks = [];
    const re = /```([a-z]*)\n?([\s\S]*?)```/g;
    let m;
    while ((m = re.exec(content))) {
      blocks.push({ lang: m[1], code: m[2].trim() });
    }
    return blocks;
  }
</script>

<div class="ai-panel">
  <div class="ai-header">
    <span>{$t.ai.title}</span>
    <div class="ai-header-right">
      <select bind:value={$settings.aiModel} class="model-select" title={$t.ai.model}>
        {#each MODELS as m}
          <option value={m.id}>{m.label}</option>
        {/each}
      </select>
      <button class="mini" onclick={clearChat} title={$t.ai.clear}>🗑</button>
      <button class="mini" onclick={() => showAiSidebar.set(false)}>✕</button>
    </div>
  </div>

  <div class="ai-messages">
    {#if messages.length === 0}
      <div class="ai-empty">
        <p>👋</p>
        <p>Ask me anything — I can help with code, tasks, or suggest commands.</p>
      </div>
    {/if}
    {#each messages as msg, i}
      <div class="ai-msg" class:user={msg.role === "user"}>
        <div class="bubble">
          {#if msg.role === "user"}
            <span>{msg.content}</span>
          {:else}
            <span style="white-space: pre-wrap">{msg.content}</span>
            {#each parseCodeBlocks(msg.content) as bl}
              <div class="code-block">
                <div class="code-head">
                  <span>{bl.lang || "bash"}</span>
                  <button class="mini insert" onclick={() => insertToTerminal(bl.code)}>⏎ {$t.ai.insert}</button>
                </div>
                <pre><code>{bl.code}</code></pre>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/each}
    {#if thinking}
      <div class="ai-msg">
        <div class="bubble thinking">… {$t.ai.thinking}</div>
      </div>
    {/if}
  </div>

  {#if pendingVisible}
    <div class="confirm-bar">
      <div class="confirm-text">
        <span class="confirm-icon">⚠️</span>
        <span>{$t.ai.confirm}: <code>{pendingCommand}</code></span>
      </div>
      <div class="confirm-actions">
        <button class="pixel-btn primary" onclick={confirmCommand}>{$t.ai.insert}</button>
        <button class="pixel-btn" onclick={dismissConfirm}>✕</button>
      </div>
    </div>
  {/if}

  <div class="ai-input">
    <textarea
      bind:value={input}
      placeholder={$t.ai.placeholder}
      rows="2"
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          send();
        }
      }}
    ></textarea>
    <button class="pixel-btn primary send" onclick={send} disabled={thinking || !input.trim()}>
      {$t.ai.send}
    </button>
  </div>
</div>

<style>
  .ai-panel {
    width: 340px;
    flex-shrink: 0;
    background: var(--bg-panel);
    border-left: 1px solid var(--border-soft);
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .ai-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-soft);
    font-weight: 600;
    font-size: 13.5px;
  }
  .ai-header-right {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .model-select {
    max-width: 130px;
    font-size: 11px;
    padding: 3px 6px;
  }
  .mini {
    font-size: 12px;
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text-dim);
  }
  .mini:hover {
    color: var(--text);
  }
  .ai-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ai-empty {
    color: var(--text-faint);
    text-align: center;
    font-size: 13px;
    margin-top: 60px;
    padding: 0 20px;
    line-height: 1.6;
  }
  .ai-msg {
    display: flex;
  }
  .ai-msg.user {
    justify-content: flex-end;
  }
  .bubble {
    max-width: 85%;
    padding: 9px 12px;
    border-radius: 12px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    font-size: 13px;
    line-height: 1.5;
    user-select: text;
    overflow-wrap: break-word;
  }
  .ai-msg.user .bubble {
    background: var(--accent-soft);
    border-color: var(--accent-dim);
  }
  .bubble.thinking {
    color: var(--text-faint);
    font-style: italic;
  }
  .code-block {
    margin-top: 8px;
    background: #0e1119;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .code-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 8px;
    background: var(--bg);
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .code-head .insert {
    font-size: 10px;
    color: var(--accent);
  }
  .code-block pre {
    padding: 8px;
    overflow-x: auto;
    font-size: 11.5px;
    font-family: var(--font-mono);
    user-select: text;
  }
  .confirm-bar {
    border-top: 1px solid var(--warn);
    background: rgba(255, 210, 110, 0.08);
    padding: 10px 12px;
  }
  .confirm-text {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    font-size: 12px;
    margin-bottom: 8px;
    color: var(--text-dim);
  }
  .confirm-text code {
    background: var(--bg-elev);
    padding: 1px 5px;
    border-radius: 4px;
    font-size: 11px;
    color: var(--warn);
    word-break: break-all;
  }
  .confirm-actions {
    display: flex;
    gap: 6px;
  }
  .ai-input {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-soft);
  }
  .ai-input textarea {
    flex: 1;
    resize: none;
    min-height: 40px;
    font-size: 13px;
    line-height: 1.4;
  }
  .send {
    align-self: flex-end;
  }
</style>
