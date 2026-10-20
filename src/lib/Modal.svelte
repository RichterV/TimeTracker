<script lang="ts">
  import type { Snippet } from "svelte";

  let { titulo, onfechar, children }: { titulo: string; onfechar?: () => void; children: Snippet } =
    $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onfechar?.()} />

<div class="fundo" role="presentation" onclick={(e) => e.target === e.currentTarget && onfechar?.()}>
  <div class="modal cartao" role="dialog" aria-modal="true" aria-label={titulo}>
    <h2>{titulo}</h2>
    {@render children()}
  </div>
</div>

<style>
  .fundo {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }

  .modal {
    width: min(460px, calc(100vw - 32px));
    padding: 20px;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
  }

  h2 {
    margin: 0 0 16px;
    font-size: 17px;
    font-weight: 600;
  }
</style>
