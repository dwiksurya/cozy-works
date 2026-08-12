<script>
  import { petState, avatarState, timeOfDay, pomodoro } from "../stores.js";
  import { onMount } from "svelte";

  export let size = 96;
  export let showLabel = false;

  // ---- Pixel art definitions: each animal is a 16x16 grid, colors per char
  // '.' transparent, 'k' outline, others fill
  const sprites = {
    cat: {
      idle: [
        "................",
        "......kkkk......",
        "....kkkkkkkk....",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwkwwwwwwkwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwppppwwkk..",
        "..kkwwppppwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "....kk....kk....",
        "................",
      ],
      happy: [
        "................",
        "......kkkk......",
        "....kkkkkkkk....",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwppppwwkk..",
        "..kkwwppppwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "....kk....kk....",
        "................",
      ],
      sleep: [
        "................",
        "................",
        "......kkkk......",
        "....kkkkkkkk....",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        "..kkwwppppwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        "................",
        "................",
        "................",
        "................",
      ],
    },
    rabbit: {
      idle: [
        "................",
        "..kk........kk..",
        "..kww......wwk..",
        "..kww......wwk..",
        "..kkkkkkkkkkkk..",
        "...kkkkkkkkkk...",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "..kkwwwwwwwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "................",
      ],
      happy: [
        "................",
        "..kk........kk..",
        "..kww......wwk..",
        "..kww......wwk..",
        "..kkkkkkkkkkkk..",
        "...kkkkkkkkkk...",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "..kkwwwwwwwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "................",
      ],
      sleep: [
        "................",
        "................",
        "................",
        "..kk........kk..",
        "..kww......wwk..",
        "..kkkkkkkkkkkk..",
        "...kkkkkkkkkk...",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        "..kkwwwwwwwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        "................",
        "................",
        "................",
        "................",
      ],
    },
    fox: {
      idle: [
        "................",
        "...kk......kk...",
        "..kork....krok..",
        "..korkkkkkkrok..",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "..kkwwwwwwwwkk..",
        "..kkkwwwwwwkkk..",
        "...kkkkkkkkkk...",
        "....kk....kk....",
        "................",
        "................",
      ],
      happy: [
        "................",
        "...kk......kk...",
        "..kork....krok..",
        "..korkkkkkkrok..",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "..kkwwwwwwwwkk..",
        "..kkkwwwwwwkkk..",
        "...kkkkkkkkkk...",
        "....kk....kk....",
        "................",
        "................",
      ],
      sleep: [
        "................",
        "................",
        "................",
        "...kk......kk...",
        "..kork....krok..",
        "..korkkkkkkrok..",
        "...kkkkkkkkkk...",
        "..kkkwwwwwwkkk..",
        ".kkwwwwwwwwwwkk.",
        "..kkkwwwwwwkkk..",
        "...kkkkkkkkkk...",
        "................",
        "................",
        "................",
        "................",
        "................",
      ],
    },
    dog: {
      idle: [
        "................",
        "..kkkk....kkkk..",
        ".kkwwkk..kkwwkk.",
        ".kkwwkkkkkkwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "....kk....kk....",
        "................",
      ],
      happy: [
        "................",
        "..kkkk....kkkk..",
        ".kkwwkk..kkwwkk.",
        ".kkwwkkkkkkwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        "...kkwwwwwwkk...",
        "....kkkkkkkk....",
        ".....kk..kk.....",
        "....kk....kk....",
        "................",
      ],
      sleep: [
        "................",
        "................",
        "................",
        "..kkkk....kkkk..",
        ".kkwwkk..kkwwkk.",
        ".kkkwwwwwwwwkkk.",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkkwwwwwwwwkkk.",
        "...kkkkkkkkkk...",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
      ],
    },
    tanuki: {
      idle: [
        "................",
        "....kk....kk....",
        "...kbbk..kbbk...",
        "..kbbbbkkbbbbk..",
        "..kbbbbbbbbbbk..",
        "..kkkkkkkkkkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkwwbbbbbbwwkk.",
        "..kkwwbbbbwwkk..",
        "..kkkwwwwwwkkk..",
        "...kkkkkkkkkk...",
        "....kk....kk....",
        "................",
        "................",
      ],
      happy: [
        "................",
        "....kk....kk....",
        "...kbbk..kbbk...",
        "..kbbbbkkbbbbk..",
        "..kbbbbbbbbbbk..",
        "..kkkkkkkkkkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        ".kkwwwwwwwwwwkk.",
        ".kkwwbbbbbbwwkk.",
        "..kkwwbbbbwwkk..",
        "..kkkwwwwwwkkk..",
        "...kkkkkkkkkk...",
        "....kk....kk....",
        "................",
        "................",
      ],
      sleep: [
        "................",
        "................",
        "................",
        "....kk....kk....",
        "...kbbk..kbbk...",
        "..kkkkkkkkkkkk..",
        "..kkwwwwwwwwkk..",
        ".kkwwwwwwwwwwkk.",
        "..kkkwwwwwwwwkkk",
        "...kkkkkkkkkk...",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
      ],
    },
  };

  // color map
  const colors = {
    k: "#2a2f3e",
    w: "#f0ead8",
    p: "#ffb3a0",
    b: "#5a6577",
    o: "#ff9d5c",
    r: "#ffb380",
  };

  let canvasEl;
  let animFrame;
  let frame = 0;

  function currentSprite() {
    const $pet = $petState;
    const animal = $pet.animal || "cat";
    const anim = $pet.animation || "idle";
    const sheet = sprites[animal] || sprites.cat;
    return sheet[anim] || sheet.idle;
  }

  function draw() {
    const canvas = canvasEl;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    const scale = size / 16;
    canvas.width = size;
    canvas.height = size;
    ctx.clearRect(0, 0, size, size);
    ctx.imageSmoothingEnabled = false;

    const sprite = currentSprite();
    const anim = $petState.animation || "idle";
    const offset = Math.sin(frame / 12) * (frame % 24 < 12 ? 0.5 : -0.5);

    for (let y = 0; y < 16; y++) {
      const row = sprite[y] || "................";
      for (let x = 0; x < 16; x++) {
        const ch = row[x];
        if (ch === "." || ch === undefined) continue;
        const col = colors[ch] || "#ffffff";
        ctx.fillStyle = col;
        // bob animation
        const bob = anim === "idle" ? (frame % 40 < 20 ? 0 : -1) : 0;
        ctx.fillRect(x * scale, y * scale + bob, scale, scale);
      }
    }
    frame++;
    animFrame = requestAnimationFrame(draw);
  }

  onMount(() => {
    draw();
    return () => cancelAnimationFrame(animFrame);
  });

  // re-render when pet changes
  $: if (canvasEl) {
    draw();
  }
</script>

<div class="pet-wrap" style="width: {size}px; height: {size}px;">
  <canvas bind:this={canvasEl} class="pixel-canvas"></canvas>
  {#if showLabel}
    <span class="pet-label">{$petState.animal || "cat"}</span>
  {/if}
</div>

<style>
  .pet-wrap {
    position: relative;
    display: inline-block;
  }
  canvas {
    width: 100%;
    height: 100%;
  }
  .pet-label {
    position: absolute;
    bottom: -16px;
    left: 50%;
    transform: translateX(-50%);
    font-size: 10px;
    color: var(--text-faint);
    text-transform: capitalize;
  }
</style>
