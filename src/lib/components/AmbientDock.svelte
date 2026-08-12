<script>
  import { ambient, effectiveAmbientScene, settings } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { initAmbient, startAmbient, stopAmbient } from "../ambient.js";
  import Icon from "./Icon.svelte";

  const scenes = ["morning", "afternoon", "evening", "night", "rain", "city"];
  const icons = {
    morning: "sun",
    afternoon: "sun",
    evening: "sun-cloud",
    night: "moon",
    rain: "cloud-rain",
    city: "city",
  };
  const sceneColors = {
    morning: "#e8b93c",
    afternoon: "#f9a825",
    evening: "#e0602a",
    night: "#4a6fa5",
    rain: "#5b7db1",
    city: "#7b1fa2",
  };

  function togglePlay() {
    if ($ambient.playing) {
      stopAmbient();
      ambient.update((a) => ({ ...a, playing: false }));
    } else {
      startAmbient();
      ambient.update((a) => ({ ...a, playing: true }));
    }
  }

  function setScene(s) {
    ambient.update((a) => ({ ...a, scene: s, mode: "manual", playing: true }));
    if (!$ambient.playing) startAmbient();
  }
  function toggleAuto() {
    ambient.update((a) => ({ ...a, mode: a.mode === "auto" ? "manual" : "auto" }));
  }

  // init once
  initAmbient();
</script>

<div class="ambient-panel pixel-panel">
  <div class="ambient-head">
    <span>{$t.ambient.title}</span>
    <div class="head-btns">
      <button class="mini-btn play" class:on={$ambient.playing} onclick={togglePlay} title="play/pause">
        {$ambient.playing ? "⏸" : "▶"}
      </button>
      <button class="mini-btn" class:on={$ambient.mode === "auto"} onclick={toggleAuto} title={$t.ambient.auto}>
        {$ambient.mode === "auto" ? "AUTO" : "MAN"}
      </button>
    </div>
  </div>
  <div class="scene-grid">
    {#each scenes as s}
      <button
        class="scene-btn"
        class:active={$effectiveAmbientScene === s}
        onclick={() => setScene(s)}
        title={$t.ambient[s]}
      >
        <span class="scene-icon" style="color: {sceneColors[s]}"><Icon name={icons[s]} size={20} /></span>
        <span class="scene-name">{$t.ambient[s]}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .ambient-panel {
    padding: 10px;
    width: 190px;
  }
  .ambient-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    font-size: 12px;
    color: var(--text-dim);
    font-weight: 600;
  }
  .mini-btn {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text-dim);
  }
  .mini-btn.play {
    font-size: 10px;
    padding: 2px 8px;
  }
  .mini-btn.on {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent-dim);
  }
  .head-btns {
    display: flex;
    gap: 4px;
  }
  .scene-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }
  .scene-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 2px;
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 10px;
  }
  .scene-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .scene-btn.active {
    background: var(--accent-soft);
    border-color: var(--accent-dim);
    color: var(--accent);
  }
  .scene-icon {
    font-size: 16px;
  }
  .scene-name {
    font-size: 9.5px;
  }
</style>
