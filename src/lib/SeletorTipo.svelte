<script lang="ts">
  import { estado } from "./estado.svelte";
  import IconeTipo from "./IconeTipo.svelte";

  let {
    value = $bindable(),
    onchange,
    compacto = false,
  }: { value: number | null; onchange?: (id: number) => void; compacto?: boolean } = $props();

  let aberto = $state(false);
  let raiz: HTMLDivElement;

  const atual = $derived(estado.tipos.find((t) => t.id === value));

  function escolher(id: number) {
    value = id;
    aberto = false;
    onchange?.(id);
  }
</script>

<svelte:window onclick={(e) => aberto && !raiz.contains(e.target as Node) && (aberto = false)} />

<div class="seletor-tipo" bind:this={raiz}>
  <button type="button" class="atual" onclick={() => (aberto = !aberto)} title="Tipo da atividade">
    <IconeTipo tipoId={value} />
    {#if !compacto}<span>{atual?.nome ?? "Tipo"}</span>{/if}
    <span class="seta">▾</span>
  </button>
  {#if aberto}
    <ul class="cartao">
      {#each estado.tipos as t (t.id)}
        <li>
          <button type="button" class:ativo={t.id === value} onclick={() => escolher(t.id)}>
            <IconeTipo tipoId={t.id} />
            {t.nome}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .seletor-tipo {
    position: relative;
    display: inline-block;
  }

  .atual {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }

  .seta {
    font-size: 10px;
    color: var(--texto-2);
  }

  ul {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 20;
    list-style: none;
    margin: 0;
    padding: 4px;
    min-width: 200px;
    max-height: 230px;
    overflow-y: auto;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: none;
    text-align: left;
    padding: 6px 8px;
  }

  li button:hover,
  li button.ativo {
    background: var(--superficie-2);
  }
</style>
