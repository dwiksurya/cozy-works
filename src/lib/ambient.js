// Ambient audio engine — Web Audio synthesis for rain/city + generated music pads for time-of-day.
// No external audio assets needed; everything is synthesized procedurally.
import { ambient, effectiveAmbientScene, settings } from "./stores.js";
import { get } from "svelte/store";

let ctx = null;
let masterGain = null;
let currentNodes = [];
let started = false;
let sceneListeners = [];

function ensureCtx() {
  if (!ctx) {
    ctx = new (window.AudioContext || window.webkitAudioContext)();
    masterGain = ctx.createGain();
    masterGain.gain.value = get(settings).ambientVolume || 0.4;
    masterGain.connect(ctx.destination);
  }
  return ctx;
}

function stopAll() {
  for (const n of currentNodes) {
    try {
      if (n.stop) n.stop();
      if (n.disconnect) n.disconnect();
    } catch (e) {
      /* ignore */
    }
  }
  currentNodes = [];
}

// ---- Scene builders ----

function buildRain() {
  const c = ensureCtx();
  // brown noise through a lowpass → rain
  const bufferSize = 2 * c.sampleRate;
  const buffer = c.createBuffer(1, bufferSize, c.sampleRate);
  const data = buffer.getChannelData(0);
  let last = 0;
  for (let i = 0; i < bufferSize; i++) {
    const white = Math.random() * 2 - 1;
    last = (last + 0.02 * white) / 1.02;
    data[i] = last * 3.5;
  }
  const src = c.createBufferSource();
  src.buffer = buffer;
  src.loop = true;
  const filter = c.createBiquadFilter();
  filter.type = "lowpass";
  filter.frequency.value = 900;
  const gain = c.createGain();
  gain.gain.value = 0.55;
  src.connect(filter);
  filter.connect(gain);
  gain.connect(masterGain);
  src.start();
  return [src, filter, gain];
}

function buildCity() {
  const c = ensureCtx();
  const nodes = [];
  // low rumble (traffic)
  const bufferSize = 2 * c.sampleRate;
  const buffer = c.createBuffer(1, bufferSize, c.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < bufferSize; i++) {
    data[i] = (Math.random() * 2 - 1) * 0.3;
  }
  const src = c.createBufferSource();
  src.buffer = buffer;
  src.loop = true;
  const lp = c.createBiquadFilter();
  lp.type = "lowpass";
  lp.frequency.value = 400;
  const g = c.createGain();
  g.gain.value = 0.35;
  src.connect(lp);
  lp.connect(g);
  g.connect(masterGain);
  src.start();
  nodes.push(src, lp, g);

  // occasional car horn blips (random oscillator beeps)
  const hornTimer = setInterval(() => {
    if (!started) return;
    const osc = c.createOscillator();
    const og = c.createGain();
    osc.type = "square";
    osc.frequency.value = 220 + Math.random() * 100;
    og.gain.value = 0.02;
    osc.connect(og);
    og.connect(masterGain);
    osc.start();
    osc.stop(c.currentTime + 0.15);
  }, 4000 + Math.random() * 5000);
  return nodes.concat([{ _timer: hornTimer }]);
}

function buildMusicPad(scene) {
  const c = ensureCtx();
  const nodes = [];
  // chord pads per scene (different keys/colors)
  const chords = {
    morning: [261.63, 329.63, 392.0], // C E G — bright
    afternoon: [293.66, 349.23, 440.0], // D F# A
    evening: [220.0, 261.63, 329.63], // A C E — warm
    night: [196.0, 246.94, 293.66], // G B D — calm
  };
  const chord = chords[scene] || chords.morning;
  for (const freq of chord) {
    const osc = c.createOscillator();
    const osc2 = c.createOscillator();
    const g = c.createGain();
    osc.type = "triangle";
    osc.frequency.value = freq;
    osc2.type = "sine";
    osc2.frequency.value = freq * 0.5; // sub octave
    const g2 = c.createGain();
    g2.gain.value = 0.3;
    g.gain.value = 0.06;
    osc.connect(g);
    osc2.connect(g2);
    g2.connect(g);
    g.connect(masterGain);
    osc.start();
    osc2.start();
    // slow LFO for breathing
    const lfo = c.createOscillator();
    const lfoGain = c.createGain();
    lfo.frequency.value = 0.05 + Math.random() * 0.03;
    lfoGain.gain.value = 0.02;
    lfo.connect(lfoGain);
    lfoGain.connect(g.gain);
    lfo.start();
    nodes.push(osc, osc2, g, g2, lfo, lfoGain);
  }
  return nodes;
}

// ---- public API ----

export function startAmbient() {
  const c = ensureCtx();
  if (c.state === "suspended") c.resume();
  started = true;
  playScene(get(effectiveAmbientScene));
}

export function stopAmbient() {
  started = false;
  stopAll();
  if (ctx && ctx.state === "running") {
    ctx.suspend();
  }
}

export function setVolume(v) {
  if (masterGain) masterGain.gain.value = v;
}

function playScene(scene) {
  if (!started) return;
  stopAll();
  const c = ensureCtx();
  if (c.state === "suspended") c.resume();
  if (scene === "rain") currentNodes = buildRain();
  else if (scene === "city") currentNodes = buildCity();
  else currentNodes = buildMusicPad(scene);
}

// subscribe to scene changes (auto/manual)
export function initAmbient() {
  effectiveAmbientScene.subscribe((scene) => {
    if (started) playScene(scene);
  });
  settings.subscribe((s) => {
    if (masterGain) masterGain.gain.value = s.ambientVolume || 0.4;
  });
}

export function isAmbientActive() {
  return started;
}
