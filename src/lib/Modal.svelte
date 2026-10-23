<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    titulo,
    onfechar,
    largura = 460,
    children,
  }: { titulo: string; onfechar?: () => void; largura?: number; children: Snippet } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onfechar?.()} />

<div class="fundo" role="presentation" onclick={(e) => e.target === e.currentTarget && onfechar?.()}>
  <div class="modal cartao" role="dialog" aria-modal="true" aria-label={titulo} style:--largura="{largura}px">
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
    width: min(var(--largura), calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow-y: auto;
    padding: 20px;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
  }

  h2 {
    margin: 0 0 16px;
    font-size: 17px;
    font-weight: 600;
  }
</style>
