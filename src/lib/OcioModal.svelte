<script lang="ts">
  import { api, type Ociosidade } from "./api";
  import { estado } from "./estado.svelte";
  import Modal from "./Modal.svelte";
  import SeletorAtividade from "./SeletorAtividade.svelte";
  import { fmtDur, fmtHora } from "./tempo";

  let { ocio }: { ocio: Ociosidade } = $props();

  let atribuindo = $state(false);
  let atividadeId = $state<number | null>(null);
  let erro = $state("");

  async function resolver(acao: "manter" | "descartar" | "atribuir") {
    try {
      await api.resolverOciosidade(acao, atividadeId ?? undefined);
    } catch (e) {
      erro = String(e);
    }
  }
</script>

<Modal titulo="Você estava ausente">
  <p>
    Sem atividade de {fmtHora(ocio.inicio)} a {fmtHora(ocio.fim)}
    (<strong>{fmtDur(ocio.fim - ocio.inicio)}</strong>) com o timer rodando
    {#if estado.rodando}em <strong>{estado.rodando.atividade}</strong>{/if}.
  </p>
  <p class="secundario">O que fazer com esse tempo?</p>

  {#if atribuindo}
    <div class="atribuir">
      <SeletorAtividade bind:value={atividadeId} />
      <button onclick={() => (atribuindo = false)}>Voltar</button>
      <button class="primario" disabled={atividadeId === null} onclick={() => resolver("atribuir")}>Atribuir</button>
    </div>
  {:else}
    <div class="acoes">
      <button onclick={() => resolver("manter")}>Manter</button>
      <button onclick={() => (atribuindo = true)}>Atribuir a outra atividade</button>
      <button class="primario" onclick={() => resolver("descartar")}>Descartar</button>
    </div>
  {/if}
  {#if erro}<p class="erro">{erro}</p>{/if}
</Modal>

<style>
  p {
    margin: 0 0 10px;
    line-height: 1.5;
  }

  .acoes,
  .atribuir {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  .atribuir :global(select) {
    flex: 1;
  }
</style>
