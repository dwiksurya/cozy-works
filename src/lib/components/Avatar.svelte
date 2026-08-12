<script>
  import { avatarState } from "../stores.js";

  export let size = 56;

  // Simple pixel avatar: head + body with mood-dependent expression
  let canvasEl;

  function draw() {
    const canvas = canvasEl;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    const scale = size / 12;
    canvas.width = size;
    canvas.height = size;
    ctx.clearRect(0, 0, size, size);
    ctx.imageSmoothingEnabled = false;

    const mood = $avatarState.mood || "idle";

    // body (shoulders) — warm color
    ctx.fillStyle = "#4a5570";
    ctx.fillRect(2 * scale, 8 * scale, 8 * scale, 3 * scale);

    // head
    ctx.fillStyle = "#ffd9b8";
    ctx.fillRect(2 * scale, 2 * scale, 8 * scale, 7 * scale);

    // hair (top)
    ctx.fillStyle = "#8ea8ff";
    ctx.fillRect(2 * scale, 1 * scale, 8 * scale, 2 * scale);
    ctx.fillRect(2 * scale, 1 * scale, 2 * scale, 4 * scale);
    ctx.fillRect(8 * scale, 1 * scale, 2 * scale, 4 * scale);

    // eyes — mood dependent
    ctx.fillStyle = "#2a2f3e";
    if (mood === "sleep") {
      ctx.fillRect(3 * scale, 4 * scale, 2 * scale, 1 * scale);
      ctx.fillRect(7 * scale, 4 * scale, 2 * scale, 1 * scale);
    } else if (mood === "focus") {
      ctx.fillRect(3 * scale, 4 * scale, 2 * scale, 2 * scale);
      ctx.fillRect(7 * scale, 4 * scale, 2 * scale, 2 * scale);
    } else {
      ctx.fillRect(3 * scale, 4 * scale, 2 * scale, 2 * scale);
      ctx.fillRect(7 * scale, 4 * scale, 2 * scale, 2 * scale);
    }

    // mouth — mood dependent
    ctx.fillStyle = "#2a2f3e";
    if (mood === "happy" || mood === "focus") {
      ctx.fillRect(4 * scale, 7 * scale, 4 * scale, 1 * scale);
    } else if (mood === "sleep") {
      // small o
      ctx.fillRect(5 * scale, 7 * scale, 2 * scale, 1 * scale);
    } else {
      ctx.fillRect(5 * scale, 7 * scale, 2 * scale, 1 * scale);
    }
  }

  $: if (canvasEl) draw();
</script>

<canvas bind:this={canvasEl} class="pixel-canvas avatar"></canvas>

<style>
  .avatar {
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
  }
</style>
