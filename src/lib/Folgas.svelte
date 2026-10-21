<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Folga } from "./api";
  import { estado } from "./estado.svelte";
  import { paraInputData } from "./tempo";

  let folgas = $state<Folga[]>([]);
  let inicio = $state(paraInputData(new Date()));
  let fim = $state(paraInputData(new Date()));
  let descricao = $state("");
  let erro = $state("");

  const carregar = async () => (folgas = await api.listarFolgas());
  onMount(carregar);

  $effect(() => {
    if (fim < inicio) fim = inicio;
  });

  async function adicionar(e: SubmitEvent) {
    e.preventDefault();
    erro = "";
    try {
      await api.criarFolga(inicio, fim, descricao);
      descricao = "";
      await carregar();
    } catch (x) {
      erro = String(x);
    }
  }

  async function excluir(f: Folga) {
    if (!(await estado.confirmar(`Excluir "${f.descricao || "Folga"}" (${fmt(f.inicio)} a ${fmt(f.fim)})?`, "Excluir"))) return;
    await api.excluirFolga(f.id);
    await carregar();
  }

  const fmt = (iso: string) => iso.split("-").reverse().join("/");
</script>

<section class="cartao">
  <h2>Folgas e férias</h2>
  <p class="dica secundario">Dias sem meta, sem lembrete e sem expediente no calendário.</p>

  <form onsubmit={adicionar}>
    <input type="date" bind:value={inicio} required />
    <span class="secundario">até</span>
    <input type="date" bind:value={fim} min={inicio} required />
    <input class="desc" bind:value={descricao} placeholder="Descrição (ex.: Férias)" />
    <button type="submit">Adicionar</button>
  </form>
  {#if erro}<p class="erro">{erro}</p>{/if}

  <ul>
    {#each folgas as f (f.id)}
      <li>
        <span class="num">{fmt(f.inicio)}{f.fim !== f.inicio ? ` a ${fmt(f.fim)}` : ""}</span>
        <span class="nome">{f.descricao || "Folga"}</span>
        <button class="icone perigo" title="Excluir" onclick={() => excluir(f)}>✕</button>
      </li>
    {:else}
      <li class="secundario vazio">Nenhuma folga cadastrada.</li>
    {/each}
  </ul>
</section>

<style>
  section {
    padding: 14px 16px;
  }

  h2 {
    margin: 0 0 4px;
    font-size: 15px;
    font-weight: 600;
  }

  .dica {
    margin: 0 0 10px;
    font-size: 12px;
  }

  form {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .desc {
    flex: 1;
    min-width: 140px;
  }

  ul {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
    border-bottom: 1px solid var(--borda);
  }

  li:last-child {
    border-bottom: none;
  }

  .nome {
    flex: 1;
  }

  .vazio {
    font-size: 12px;
  }

  .erro {
    margin: 6px 0 0;
  }
</style>
