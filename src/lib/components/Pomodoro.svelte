<script>
  import { pomodoro, settings, petState, avatarState, toast } from "../stores.js";
  import { t } from "../i18n-store.js";
  import { fmtTime, todayStr } from "../i18n-store.js";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { invoke } from "@tauri-apps/api/core";
  import Database from "@tauri-apps/plugin-sql";

  let timerId;

  const phases = {
    focus: { key: "pomodoro.focus", color: "var(--accent)" },
    short: { key: "pomodoro.shortBreak", color: "var(--blue)" },
    long: { key: "pomodoro.longBreak", color: "var(--purple)" },
  };

  function phaseLabel() {
    const $t = get(t);
    return $t.pomodoro[phases[$pomodoro.phase].key.split(".")[1]];
  }

  async function loadStats() {
    try {
      const db = await Database.load("sqlite:cozy.db");
      const rows = await db.select("SELECT * FROM pomodoro_stats WHERE date = $1", [todayStr()]);
      if (rows.length) {
        pomodoro.update((p) => ({
          ...p,
          completedToday: rows[0].sessions || 0,
          todayMinutes: rows[0].focus_minutes || 0,
        }));
      }
    } catch (e) {
      console.warn("loadStats", e);
    }
  }

  async function recordSession(minutes) {
    try {
      const db = await Database.load("sqlite:cozy.db");
      await db.execute(
        `INSERT INTO pomodoro_stats (date, focus_minutes, sessions) VALUES ($1, $2, 1)
         ON CONFLICT(date) DO UPDATE SET
           focus_minutes = focus_minutes + $2,
           sessions = sessions + 1`,
        [todayStr(), minutes]
      );
      loadStats();
    } catch (e) {
      console.warn("recordSession", e);
    }
  }

  function startTimer() {
    if ($pomodoro.running) return;
    const $s = get(settings);
    let dur;
    if ($pomodoro.phase === "focus") dur = $s.focusMinutes * 60;
    else if ($pomodoro.phase === "short") dur = $s.shortBreak * 60;
    else dur = $s.longBreak * 60;
    pomodoro.update((p) => ({ ...p, running: true, remaining: dur, total: dur }));

    // pet & avatar mood
    if ($pomodoro.phase === "focus") {
      petState.update((p) => ({ ...p, mood: "focus", animation: "idle" }));
      avatarState.update((a) => ({ ...a, mood: "focus" }));
    }
  }

  function pauseTimer() {
    pomodoro.update((p) => ({ ...p, running: false }));
    petState.update((p) => ({ ...p, mood: "idle" }));
    avatarState.update((a) => ({ ...a, mood: "idle" }));
  }

  function resetTimer() {
    pomodoro.update((p) => ({ ...p, running: false }));
    const $s = get(settings);
    const dur = $pomodoro.phase === "focus" ? $s.focusMinutes * 60 : $s.shortBreak * 60;
    pomodoro.update((p) => ({ ...p, remaining: dur, total: dur }));
  }

  function skipPhase() {
    pomodoro.update((p) => {
      let next = "focus";
      if (p.phase === "focus") next = "short";
      else if (p.phase === "short") next = "focus";
      else next = "focus";
      return { ...p, phase: next, running: false };
    });
  }

  function onTick() {
    const $s = get(settings);
    pomodoro.update((p) => {
      if (!p.running) return p;
      let remaining = p.remaining - 1;
      if (remaining <= 0) {
        // phase complete
        if (p.phase === "focus") {
          recordSession($s.focusMinutes);
          if ($s.soundOn) playBeep();
          toast(get(t).notif.pomodoroDone, "info");
          petState.update((pp) => ({ ...pp, mood: "happy", animation: "happy" }));
          avatarState.update((a) => ({ ...a, mood: "happy" }));
          setTimeout(() => petState.update((pp) => ({ ...pp, mood: "idle", animation: "idle" })), 3000);
          // decide break
          const sessionsToday = p.completedToday + 1;
          const phase = sessionsToday % $s.longEvery === 0 ? "long" : "short";
          const dur = phase === "long" ? $s.longBreak * 60 : $s.shortBreak * 60;
          if ($s.autoStartBreak) {
            return { ...p, phase, remaining: dur, total: dur, running: true, completedToday: sessionsToday };
          }
          return { ...p, phase, remaining: dur, total: dur, running: false, completedToday: sessionsToday };
        } else {
          // break done
          if ($s.soundOn) playBeep();
          toast(get(t).notif.breakDone, "info");
          const dur = $s.focusMinutes * 60;
          petState.update((pp) => ({ ...pp, mood: "focus", animation: "idle" }));
          avatarState.update((a) => ({ ...a, mood: "focus" }));
          if ($s.autoStartFocus) {
            return { ...p, phase: "focus", remaining: dur, total: dur, running: true };
          }
          return { ...p, phase: "focus", remaining: dur, total: dur, running: false };
        }
      }
      return { ...p, remaining };
    });
  }

  function playBeep() {
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = 880;
      gain.gain.value = 0.15;
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.4);
    } catch (e) {
      /* ignore */
    }
  }

  // progress ring
  function progressPct() {
    const p = $pomodoro;
    return p.total > 0 ? Math.round(((p.total - p.remaining) / p.total) * 100) : 0;
  }

  onMount(async () => {
    await loadStats();
    timerId = setInterval(onTick, 1000);
  });

  onDestroy(() => clearInterval(timerId));
</script>

<div class="pomodoro-view">
  <div class="pomodoro-card pixel-panel">
    <div class="phase-tag" style="color: {phases[$pomodoro.phase].color}">
      {phaseLabel()}
    </div>

    <div class="timer-wrap">
      <svg viewBox="0 0 120 120" class="ring">
        <circle cx="60" cy="60" r="52" class="ring-bg" />
        <circle
          cx="60"
          cy="60"
          r="52"
          class="ring-fg"
          style="stroke: {phases[$pomodoro.phase].color}; stroke-dasharray: {2 * Math.PI * 52}; stroke-dashoffset: {2 * Math.PI * 52 * (1 - progressPct() / 100)};"
        />
      </svg>
      <div class="timer-text">{fmtTime($pomodoro.remaining)}</div>
    </div>

    <div class="controls">
      {#if $pomodoro.running}
        <button class="pixel-btn primary" onclick={pauseTimer}>{$t.pomodoro.pause}</button>
      {:else}
        <button class="pixel-btn primary" onclick={startTimer}>{$t.pomodoro.start}</button>
      {/if}
      <button class="pixel-btn" onclick={resetTimer}>{$t.pomodoro.reset}</button>
      <button class="pixel-btn" onclick={skipPhase}>{$t.pomodoro.skip}</button>
    </div>
  </div>

  <div class="stats-card pixel-panel">
    <h3>{$t.pomodoro.stats}</h3>
    <div class="stats-row">
      <div class="stat">
        <span class="stat-num">{$pomodoro.completedToday}</span>
        <span class="stat-label">{$t.pomodoro.completed}</span>
      </div>
      <div class="stat">
        <span class="stat-num">{$pomodoro.todayMinutes}</span>
        <span class="stat-label">{$t.pomodoro.focusTime}</span>
      </div>
    </div>
    <p class="hint">{$t.pomodoro.settingsHint}</p>
  </div>
</div>

<style>
  .pomodoro-view {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
    padding: 24px;
    height: 100%;
    align-content: flex-start;
  }
  .pomodoro-card {
    padding: 28px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    min-width: 300px;
  }
  .phase-tag {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }
  .timer-wrap {
    position: relative;
    width: 180px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ring {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
  }
  .ring-bg {
    fill: none;
    stroke: var(--border);
    stroke-width: 8;
  }
  .ring-fg {
    fill: none;
    stroke-width: 8;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.3s linear;
  }
  .timer-text {
    font-size: 38px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
  }
  .controls {
    display: flex;
    gap: 8px;
  }
  .stats-card {
    padding: 20px;
    min-width: 260px;
  }
  .stats-card h3 {
    font-size: 14px;
    margin-bottom: 14px;
    color: var(--text-dim);
  }
  .stats-row {
    display: flex;
    gap: 16px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .stat-num {
    font-size: 30px;
    font-weight: 700;
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .stat-label {
    font-size: 11px;
    color: var(--text-faint);
    max-width: 120px;
  }
  .hint {
    margin-top: 14px;
    font-size: 11px;
    color: var(--text-faint);
  }
</style>
