<script>
  import { toastMsg } from "../stores.js";
</script>

{#if $toastMsg}
  <div class="notif-toast" class:error={$toastMsg.type === "error"}>
    <span>{$toastMsg.text}</span>
    <button onclick={() => toastMsg.set(null)}>✕</button>
  </div>
{/if}

<style>
  .notif-toast span {
    flex: 1;
  }
  .notif-toast button {
    color: var(--text-dim);
    font-size: 14px;
  }
</style>
