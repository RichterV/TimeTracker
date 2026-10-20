<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount, untrack } from "svelte";
  import { api, type Anexo } from "./api";
  import { estado } from "./estado.svelte";
  import { STATUS } from "./status";
  import SeletorTipo from "./SeletorTipo.svelte";
  import Comentarios from "./Comentarios.svelte";
  import { fmtDur, fmtHora } from "./tempo";

  let { id }: { id: number } = $props();

  const info = $derived(estado.buscarAtividade(id));
  const rodandoAqui = $derived(estado.rodando?.atividade_id === id);

  let nome = $state("");
  let anexos = $state<Anexo[]>([]);
  let total = $state(0);
  let totalEm = $state(0);
  let arrastando = $state(false);
  let excluindo = $state<number | null>(null);
  let mostrarIndice = $state(false);
  let erro = $state("");
  let comentarios = $state<Comentarios>();

  $effect(() => {
    const atual = id;
    untrack(() => carregar(atual));
  });

  $effect(() => {
    estado.versao;
    untrack(() => atualizarTotal());
  });

  async function carregar(atual: number) {
    nome = estado.buscarAtividade(atual)?.atividade.nome ?? "";
    erro = "";
    anexos = await api.listarAnexos(atual);
    await atualizarTotal();
  }

  async function atualizarTotal() {
    total = await api.totalAtividade(id);
    totalEm = estado.agora;
  }

  async function executar(f: () => Promise<unknown>) {
    erro = "";
    try {
      await f();
    } catch (e) {
      erro = String(e);
    }
  }

  function salvarNome() {
    const a = info?.atividade;
    if (a && nome.trim() && nome.trim() !== a.nome) executar(() => api.renomearAtividade(id, nome.trim()));
  }

  /** Print colado fora de um campo de texto vai para o editor de comentário ativo. */
  function colar(e: ClipboardEvent) {
    const foco = document.activeElement;
    if (foco instanceof HTMLTextAreaElement || foco instanceof HTMLInputElement) return;
    const arquivo = [...(e.clipboardData?.items ?? [])].find((i) => i.type.startsWith("image/"))?.getAsFile();
    if (!arquivo) return;
    e.preventDefault();
    comentarios?.colarArquivo(arquivo);
  }

  async function excluirAnexo(a: Anexo) {
    if (excluindo !== a.id) {
      excluindo = a.id;
      setTimeout(() => excluindo === a.id && (excluindo = null), 3000);
      return;
    }
    excluindo = null;
    await executar(async () => {
      await api.excluirAnexo(a.id);
      anexos = anexos.filter((x) => x.id !== a.id);
    });
  }

  async function excluirAtividade() {
    const ok = await estado.confirmar(`Excluir a atividade "${info?.atividade.nome}" e seus anexos?`, "Excluir");
    if (!ok) return;
    await executar(async () => {
      await api.excluirAtividade(id);
      await estado.recarregarProjetos();
      fechar();
    });
  }

  function fechar() {
    salvarNome();
    estado.atividadeAberta = null;
  }

  onMount(() => {
    const desligar = getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === "enter" || p.type === "over") arrastando = true;
      else if (p.type === "leave") arrastando = false;
      else if (p.type === "drop") {
        arrastando = false;
        if (p.paths.length) comentarios?.anexarCaminhos(p.paths);
      }
    });
    return () => desligar.then((f) => f());
  });

  const tamanho = (b: number) => (b > 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(b / 1024)} KB`);
</script>

<svelte:window
  onpaste={colar}
  onkeydown={(e) => e.key === "Escape" && !estado.confirmacao && fechar()}
/>

<div class="fundo" role="presentation" onclick={fechar}></div>

<aside class:arrastando>
  {#if info}
    <header>
      <span class="ponto" style:background={info.projeto.cor}></span>
      <span class="secundario">{info.projeto.nome}</span>
      <button class="icone fechar" onclick={fechar} title="Fechar">✕</button>
    </header>

    <input class="nome" bind:value={nome} onblur={salvarNome} />

    <div class="tipo">
      <span class="secundario">Tipo</span>
      <SeletorTipo
        value={info.atividade.tipo_id}
        onchange={(t) => executar(() => api.definirTipo(id, t))}
      />
    </div>

    <div class="status">
      {#each STATUS as s}
        <button
          class:ativo={info.atividade.status === s.id}
          style:--cor={s.cor}
          onclick={() => executar(() => api.definirStatus(id, s.id))}
        >
          {s.nome}
        </button>
      {/each}
    </div>

    <div class="tempo">
      <div>
        <span class="secundario">Tempo total</span>
        <strong class="num">{fmtDur(total + (rodandoAqui ? estado.agora - totalEm : 0))}</strong>
      </div>
      {#if rodandoAqui}
        <button onclick={() => api.parar()}>Parar timer</button>
      {:else}
        <button class="primario" onclick={() => estado.iniciarAtividade(id)}>▶ Iniciar timer</button>
      {/if}
    </div>

    <span class="secao secundario">Comentários</span>
    <Comentarios
      bind:this={comentarios}
      atividadeId={id}
      {anexos}
      onanexos={(novos) => (anexos = [...anexos, ...novos])}
      onanexosremovidos={(cid) => (anexos = anexos.filter((a) => a.comentario_id !== cid))}
      onerro={(m) => (erro = m)}
    />

    <button class="titulo-anexos" onclick={() => (mostrarIndice = !mostrarIndice)}>
      <span class="secundario">{mostrarIndice ? "▾" : "▸"} Todos os anexos ({anexos.length})</span>
    </button>

    {#if mostrarIndice}
      <div class="anexos">
        {#each anexos as a (a.id)}
          <div class="anexo" title="{a.nome}&#10;{tamanho(a.tamanho)} · {fmtHora(a.criado)}">
            <button class="miniatura" onclick={() => executar(() => api.abrirAnexo(a.id))}>
              {#if a.tipo.startsWith("image/")}
                <img src={convertFileSrc(a.caminho)} alt={a.nome} loading="lazy" />
              {:else}
                <span class="pdf">PDF</span>
              {/if}
            </button>
            <div class="rodape-anexo">
              <span class="nome-anexo">{a.nome}</span>
              <button class="icone" class:perigo={excluindo === a.id} onclick={() => excluirAnexo(a)}>
                {excluindo === a.id ? "Excluir?" : "✕"}
              </button>
            </div>
          </div>
        {:else}
          <p class="secundario vazio-anexos">Nenhum anexo.</p>
        {/each}
      </div>
    {/if}

    {#if erro}<p class="erro">{erro}</p>{/if}

    <footer>
      <button class="icone perigo" onclick={excluirAtividade}>Excluir atividade</button>
    </footer>
  {/if}
</aside>

<style>
  .fundo {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.2);
    z-index: 5;
  }

  aside {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(520px, 100vw);
    background: var(--superficie);
    border-left: 1px solid var(--borda);
    box-shadow: -8px 0 30px rgb(0 0 0 / 0.15);
    z-index: 6;
    padding: 16px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  aside.arrastando {
    outline: 3px dashed var(--destaque);
    outline-offset: -8px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .fechar {
    margin-left: auto;
  }

  .nome {
    font-size: 18px;
    font-weight: 600;
    border-color: transparent;
    background: transparent;
    padding: 4px 6px;
    margin: 0 -6px;
  }

  .nome:hover {
    border-color: var(--borda);
  }

  .tipo {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }

  .tipo :global(button) {
    font-size: 14px;
  }

  .status {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }

  .status button {
    font-size: 12px;
    padding: 6px 4px;
  }

  .status button.ativo {
    background: var(--cor);
    border-color: var(--cor);
    color: #fff;
  }

  .tempo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--superficie-2);
    border-radius: var(--raio);
    padding: 10px 12px;
  }

  .tempo div {
    display: flex;
    flex-direction: column;
  }

  .tempo strong {
    font-size: 20px;
  }

  .secao {
    font-size: 12px;
    margin-bottom: -4px;
  }

  .titulo-anexos {
    background: none;
    border: none;
    padding: 4px 0;
    text-align: left;
    font-size: 12px;
  }

  .vazio-anexos {
    grid-column: 1 / -1;
    font-size: 12px;
    margin: 0;
  }

  .anexos {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .anexo {
    border: 1px solid var(--borda);
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .miniatura {
    height: 84px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: var(--superficie-2);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .miniatura img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .pdf {
    font-weight: 700;
    color: var(--perigo);
    font-size: 18px;
  }

  .rodape-anexo {
    display: flex;
    align-items: center;
    font-size: 11px;
    padding-left: 6px;
  }

  .nome-anexo {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .rodape-anexo button {
    font-size: 11px;
    padding: 2px 6px;
  }

  footer {
    margin-top: auto;
    padding-top: 8px;
    display: flex;
    justify-content: flex-end;
  }
</style>
