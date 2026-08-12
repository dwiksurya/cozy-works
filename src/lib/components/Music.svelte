<script>
  import { musicState, settings, toast } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icon.svelte";

  let audio = null;
  let elCurrent = null;

  function isTauri() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  async function pickFolder() {
    if (!isTauri()) {
      toast("Folder picker requires Tauri runtime", "error");
      return;
    }
    try {
      const dir = await dialogOpen({ directory: true });
      if (dir) {
        settings.update((s) => ({ ...s, musicDir: dir }));
        await scan(dir);
      }
    } catch (e) {
      console.warn(e);
    }
  }

  async function scan(dir) {
    if (!isTauri()) return;
    try {
      const files = await invoke("scan_music", { dir });
      const lib = files.map((f, i) => ({
        id: i,
        path: f.path,
        name: f.name || f.path.split("/").pop().split("\\").pop().replace(/\.(mp3|flac|wav|ogg|m4a)$/i, ""),
      }));
      musicState.update((m) => ({ ...m, library: lib, queue: lib }));
    } catch (e) {
      toast(`Scan failed: ${e}`, "error");
    }
  }

  function playTrack(track) {
    const url = `asset://localhost/${track.path}`;
    if (!audio) {
      audio = new Audio();
    }
    audio.src = url;
    audio.play();
    musicState.update((m) => ({ ...m, current: track, playing: true }));
  }

  function togglePlay() {
    if (!audio) {
      const q = $musicState.queue;
      if (q.length) playTrack(q[0]);
      return;
    }
    if ($musicState.playing) {
      audio.pause();
      musicState.update((m) => ({ ...m, playing: false }));
    } else {
      audio.play();
      musicState.update((m) => ({ ...m, playing: true }));
    }
  }

  function next() {
    const m = $musicState;
    const q = m.queue;
    if (!q.length) return;
    const idx = m.current ? q.findIndex((t) => t.id === m.current.id) : -1;
    const n = (idx + 1) % q.length;
    playTrack(q[n]);
  }

  function prev() {
    const m = $musicState;
    const q = m.queue;
    if (!q.length) return;
    const idx = m.current ? q.findIndex((t) => t.id === m.current.id) : 0;
    const n = (idx - 1 + q.length) % q.length;
    playTrack(q[n]);
  }

  function toggleShuffle() {
    musicState.update((m) => ({ ...m, shuffle: !m.shuffle }));
  }

  function toggleRepeat() {
    musicState.update((m) => ({ ...m, repeat: !m.repeat }));
  }

  function setVolume(v) {
    musicState.update((m) => ({ ...m, volume: v }));
    if (audio) audio.volume = v;
  }

  // auto-advance on end
  function onEnded() {
    const m = $musicState;
    if (m.repeat) {
      if (audio) audio.currentTime = 0, audio.play();
      return;
    }
    next();
  }

  onMount(() => {
    if ($settings.musicDir) scan($settings.musicDir);
  });

  $: if (audio) audio.volume = $musicState.volume;
  $: if (audio && $musicState.current) audio.volume = $musicState.volume;
</script>

<div class="music-view">
  <h2>{$t.music.title}</h2>

  <div class="music-toolbar">
    <button class="pixel-btn" onclick={pickFolder}><Icon name="music" size={15} /> {$t.music.addFolder}</button>
    {#if $settings.musicDir}
      <span class="dir-label">{$settings.musicDir}</span>
    {/if}
  </div>

  {#if $musicState.library.length === 0}
    <div class="empty">{#if $settings.musicDir}{$t.music.scan}{:else}{$t.music.noLibrary}{/if}</div>
  {:else}
    <div class="player pixel-panel">
      <div class="now-playing">
        <div class="np-art"><Icon name="music" size={28} /></div>
        <div class="np-info">
          <div class="np-title">{$musicState.current?.name || "—"}</div>
          <div class="np-meta">{$t.music.nowPlaying}</div>
        </div>
        <div class="np-controls">
          <button class="ctrl" onclick={prev} title="prev"><Icon name="skip-back" size={16} /></button>
          <button class="ctrl play" onclick={togglePlay}>
            {#if $musicState.playing}
              <Icon name="pause" size={16} />
            {:else}
              <Icon name="play" size={16} />
            {/if}
          </button>
          <button class="ctrl" onclick={next} title="next"><Icon name="skip-forward" size={16} /></button>
          <button class="ctrl" class:active={$musicState.shuffle} onclick={toggleShuffle} title={$t.music.shuffle}><Icon name="shuffle" size={16} /></button>
          <button class="ctrl" class:active={$musicState.repeat} onclick={toggleRepeat} title={$t.music.repeat}><Icon name="repeat" size={16} /></button>
        </div>
        <div class="np-volume">
          <input type="range" min="0" max="1" step="0.05" value={$musicState.volume} oninput={(e) => setVolume(parseFloat(e.target.value))} />
        </div>
      </div>
      <div class="queue">
        {#each $musicState.queue as track}
          <div class="queue-item" class:current={$musicState.current?.id === track.id} onclick={() => playTrack(track)}>
            <span class="qi-icon">{track.name}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .music-view {
    padding: 24px;
    height: 100%;
    overflow-y: auto;
  }
  .music-view h2 {
    font-size: 20px;
    margin-bottom: 14px;
  }
  .music-toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 16px;
  }
  .dir-label {
    font-size: 12px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }
  .player {
    padding: 18px;
  }
  .now-playing {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }
  .np-art {
    width: 52px;
    height: 52px;
    border-radius: 8px;
    background: linear-gradient(135deg, var(--music-purple), var(--blue));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    color: #fff;
    flex-shrink: 0;
    border: 2px solid var(--text);
  }
  .np-info {
    flex: 1;
    min-width: 0;
  }
  .np-title {
    font-weight: 600;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .np-meta {
    font-size: 11px;
    color: var(--text-faint);
  }
  .np-controls {
    display: flex;
    gap: 6px;
  }
  .ctrl {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ctrl.play {
    background: var(--accent-dim);
    border-color: var(--accent-dim);
    color: #0c1510;
    font-size: 15px;
  }
  .ctrl.active {
    background: var(--accent-soft);
    border-color: var(--accent-dim);
    color: var(--accent);
  }
  .np-volume {
    width: 100px;
    flex-shrink: 0;
  }
  .np-volume input {
    width: 100%;
    accent-color: var(--accent);
  }
  .queue {
    max-height: 300px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .queue-item {
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-dim);
    transition: background 0.12s, color 0.12s;
  }
  .queue-item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .queue-item.current {
    border-color: var(--accent-dim);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .empty {
    color: var(--text-faint);
    text-align: center;
    padding: 60px 0;
    font-size: 14px;
  }
</style>
