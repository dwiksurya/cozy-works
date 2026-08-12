<script>
  import { settings, toast, settingsLoaded } from "../stores.js";
  import { t, lang } from "../i18n-store.js";
  import { get } from "svelte/store";
  import { onMount } from "svelte";
  import Database from "@tauri-apps/plugin-sql";
  import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icon.svelte";

  let saved = false;
  let aiKey = "";

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

  async function pickFolder() {
    if (!isTauri()) return;
    try {
      const dir = await dialogOpen({ directory: true });
      if (dir) settings.update((s) => ({ ...s, musicDir: dir }));
    } catch (e) {
      console.warn(e);
    }
  }

  async function save() {
    if (aiKey.trim()) {
      settings.update((s) => ({ ...s, aiApiKey: aiKey.trim() }));
    }
    try {
      if (isTauri()) {
        const db = await Database.load("sqlite:cozy.db");
        const $s = get(settings);
        for (const [k, v] of Object.entries($s)) {
          if (k === "aiApiKey") continue;
          await db.execute(
            "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [k, typeof v === "object" ? JSON.stringify(v) : String(v)]
          );
        }
        if (aiKey.trim()) {
          await db.execute("INSERT INTO settings (key, value) VALUES ('aiApiKey', $1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [aiKey.trim()]);
        }
      }
      saved = true;
      toast($t.settings.saved, "info");
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      toast(String(e), "error");
    }
  }
</script>

<div class="settings-view">
  <h2>{$t.settings.title}</h2>

  <div class="settings-grid">
    <section class="set-card pixel-panel">
      <h3><Icon name="user" size={14} /> {$t.settings.language}</h3>
      <div class="set-row">
        <label>{$t.settings.language}</label>
        <select bind:value={$settings.lang} onchange={(e) => lang.set(e.target.value)}>
          <option value="en">English</option>
          <option value="id">Indonesia</option>
        </select>
      </div>
    </section>

    <section class="set-card pixel-panel">
      <h3><Icon name="clock" size={14} /> {$t.settings.pomodoro}</h3>
      <div class="set-row">
        <label>{$t.settings.focusMin}</label>
        <input type="number" min="1" max="120" bind:value={$settings.focusMinutes} />
      </div>
      <div class="set-row">
        <label>{$t.settings.shortBreakMin}</label>
        <input type="number" min="1" max="60" bind:value={$settings.shortBreak} />
      </div>
      <div class="set-row">
        <label>{$t.settings.longBreakMin}</label>
        <input type="number" min="1" max="60" bind:value={$settings.longBreak} />
      </div>
      <div class="set-row">
        <label>{$t.settings.longEvery}</label>
        <input type="number" min="1" max="10" bind:value={$settings.longEvery} />
      </div>
      <div class="set-row">
        <label>{$t.settings.autoStartBreak}</label>
        <input type="checkbox" bind:checked={$settings.autoStartBreak} />
      </div>
      <div class="set-row">
        <label>{$t.settings.autoStartFocus}</label>
        <input type="checkbox" bind:checked={$settings.autoStartFocus} />
      </div>
      <div class="set-row">
        <label>{$t.settings.sound}</label>
        <input type="checkbox" bind:checked={$settings.soundOn} />
      </div>
    </section>

    <section class="set-card pixel-panel">
      <h3><Icon name="spark" size={14} /> AI {$t.settings.ai}</h3>
      <div class="set-row">
        <label>{$t.settings.aiModel}</label>
        <select bind:value={$settings.aiModel}>
          {#each MODELS as m}
            <option value={m.id}>{m.label}</option>
          {/each}
        </select>
      </div>
      <div class="set-row">
        <label>{$t.settings.aiBaseUrl}</label>
        <input type="text" bind:value={$settings.aiBaseUrl} />
      </div>
      <div class="set-row">
        <label>API Key</label>
        <input type="password" bind:value={aiKey} placeholder="•••••••• (leave empty to keep)" />
      </div>
      <div class="set-row">
        <label>{$t.settings.aiNotifyDone}</label>
        <input type="checkbox" bind:checked={$settings.aiNotifyOnDone} />
      </div>
      <div class="set-row">
        <label>{$t.settings.aiNotifyAsk}</label>
        <input type="checkbox" bind:checked={$settings.aiNotifyOnAsk} />
      </div>
      <p class="hint">{$t.ai.apiKeyHint}</p>
    </section>

    <section class="set-card pixel-panel">
      <h3><Icon name="paw" size={14} /> {$t.settings.pet}</h3>
      <div class="set-row">
        <label>{$t.settings.choosePet}</label>
        <select bind:value={$settings.pet}>
          <option value="cat">{$t.pet.cat}</option>
          <option value="rabbit">{$t.pet.rabbit}</option>
          <option value="fox">{$t.pet.fox}</option>
          <option value="dog">{$t.pet.dog}</option>
          <option value="tanuki">{$t.pet.tanuki}</option>
        </select>
      </div>
      <div class="set-row">
        <label>{$t.settings.avatarHair}</label>
        <input type="color" bind:value={$settings.avatarHair} />
      </div>
      <div class="set-row">
        <label>{$t.settings.avatarSkin}</label>
        <input type="color" bind:value={$settings.avatarSkin} />
      </div>
    </section>

    <section class="set-card pixel-panel">
      <h3><Icon name="music" size={14} /> {$t.settings.music}</h3>
      <div class="set-row">
        <label for="musicDir">{$t.settings.musicDir}</label>
        <div class="row-inline">
          <input id="musicDir" type="text" bind:value={$settings.musicDir} placeholder="~/Music" />
          <button class="pixel-btn" onclick={pickFolder}>{$t.settings.chooseFolder}</button>
        </div>
      </div>
    </section>

    <section class="set-card pixel-panel">
      <h3><Icon name="cloud" size={14} /> {$t.settings.ambient}</h3>
      <div class="set-row">
        <label for="ambientMode">{$t.settings.ambientMode}</label>
        <select id="ambientMode" bind:value={$settings.ambientMode}>
          <option value="auto">{$t.ambient.auto}</option>
          <option value="manual">{$t.ambient.manual}</option>
        </select>
      </div>
      <div class="set-row">
        <label for="ambientVol">{$t.ambient.volume}</label>
        <input id="ambientVol" type="range" min="0" max="1" step="0.05" bind:value={$settings.ambientVolume} />
      </div>
    </section>
  </div>

  <div class="save-bar">
    <button class="pixel-btn primary" onclick={save}>{$t.settings.save}</button>
  </div>
</div>

<style>
  .settings-view {
    padding: 24px;
    overflow-y: auto;
    height: 100%;
  }
  .settings-view h2 {
    font-size: 20px;
    margin-bottom: 16px;
  }
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 14px;
  }
  .set-card {
    padding: 16px;
  }
  .set-card h3 {
    font-size: 13.5px;
    color: var(--text-dim);
    margin-bottom: 12px;
  }
  .set-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
    font-size: 13px;
  }
  .set-row label {
    color: var(--text);
    flex: 1;
  }
  .set-row input[type="text"],
  .set-row input[type="password"],
  .set-row input[type="number"],
  .set-row select {
    width: 170px;
  }
  .set-row input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .set-row input[type="color"] {
    width: 40px;
    height: 30px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elev);
    cursor: pointer;
  }
  .row-inline {
    display: flex;
    gap: 6px;
    flex: 1;
  }
  .row-inline input {
    flex: 1;
  }
  .hint {
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-faint);
  }
  .save-bar {
    margin-top: 18px;
    display: flex;
    justify-content: flex-end;
  }
</style>
