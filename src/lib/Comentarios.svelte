<script lang="ts">
  import { untrack } from "svelte";
  import { api, type Anexo, type Comentario } from "./api";
  import EditorComentario from "./EditorComentario.svelte";
  import { renderizar } from "./markdown";

  let {
    atividadeId,
    anexos,
    onanexos,
    onanexosremovidos,
    onerro,
  }: {
    atividadeId: number;
    anexos: Anexo[];
    onanexos: (novos: Anexo[]) => void;
    onanexosremovidos: (comentarioId: number) => void;
    onerro: (msg: string) => void;
  } = $props();

  let comentarios = $state<Comentario[]>([]);
  let novoTexto = $state("");
  let editando = $state<number | null>(null);
  let textoEdicao = $state("");
  let excluindo = $state<number | null>(null);
  /** Editor que recebe anexos arrastados ou colados fora do campo. */
  let alvo = $state<"novo" | "edicao">("novo");
  let editorNovo = $state<EditorComentario>();
  let editorEdicao = $state<EditorComentario>();

  const mapaAnexos = $derived(new Map(anexos.map((a) => [a.id, a])));

  $effect(() => {
    const id = atividadeId;
    untrack(async () => (comentarios = await api.listarComentarios(id)));
  });

  const editorAlvo = () => (alvo === "edicao" && editorEdicao ? editorEdicao : editorNovo);

  export const anexarCaminhos = (caminhos: string[]) => editorAlvo()?.anexarCaminhos(caminhos);
  export const colarArquivo = (arquivo: File) => editorAlvo()?.colarArquivo(arquivo);

  async function salvarNovo(texto: string) {
    try {
      comentarios = [...comentarios, await api.salvarComentario(atividadeId, null, texto)];
      novoTexto = "";
    } catch (e) {
      onerro(String(e));
    }
  }

  function editar(c: Comentario) {
    editando = c.id;
    textoEdicao = c.texto;
    alvo = "edicao";
  }

  function cancelarEdicao() {
    editando = null;
    alvo = "novo";
  }

  async function salvarEdicao(texto: string) {
    if (editando === null) return;
    try {
      const salvo = await api.salvarComentario(atividadeId, editando, texto);
      comentarios = comentarios.map((c) => (c.id === salvo.id ? salvo : c));
      cancelarEdicao();
    } catch (e) {
      onerro(String(e));
    }
  }

  async function excluir(c: Comentario) {
    if (excluindo !== c.id) {
      excluindo = c.id;
      setTimeout(() => excluindo === c.id && (excluindo = null), 3000);
      return;
    }
    excluindo = null;
    try {
      await api.excluirComentario(c.id);
      comentarios = comentarios.filter((x) => x.id !== c.id);
      onanexosremovidos(c.id);
    } catch (e) {
      onerro(String(e));
    }
  }

  /** Cliques em anexos e links dentro do texto renderizado. */
  function clique(e: MouseEvent) {
    const alvoClique = e.target as HTMLElement;
    const anexo = alvoClique.closest<HTMLElement>("[data-anexo]");
    if (anexo) return api.abrirAnexo(Number(anexo.dataset.anexo)).catch((x) => onerro(String(x)));
    const link = alvoClique.closest<HTMLAnchorElement>("a");
    if (link) {
      e.preventDefault();
      if (link.hasAttribute("data-link")) api.abrirLink(link.getAttribute("href")!).catch((x) => onerro(String(x)));
    }
  }

  const fmtQuando = (ts: number) =>
    new Date(ts * 1000).toLocaleString("pt-BR", { day: "2-digit", month: "2-digit", year: "2-digit", hour: "2-digit", minute: "2-digit" });
</script>

<div class="linha-tempo">
  {#each comentarios as c (c.id)}
    <article>
      <header>
        <span class="secundario">{fmtQuando(c.criado)}{#if c.editado} · editado{/if}</span>
        {#if editando !== c.id}
          <div class="acoes">
            <button class="icone" title="Editar" onclick={() => editar(c)}>✎</button>
            <button class="icone" class:perigo={excluindo === c.id} title="Excluir comentário e anexos dele" onclick={() => excluir(c)}>
              {excluindo === c.id ? "Excluir?" : "✕"}
            </button>
          </div>
        {/if}
      </header>
      {#if editando === c.id}
        <EditorComentario
          bind:this={editorEdicao}
          bind:texto={textoEdicao}
          {atividadeId}
          rotuloSalvar="Salvar"
          autofoco
          onsalvar={salvarEdicao}
          oncancelar={cancelarEdicao}
          {onanexos}
          onfoco={() => (alvo = "edicao")}
          {onerro}
        />
      {:else}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="corpo" onclick={clique}>{@html renderizar(c.texto, mapaAnexos)}</div>
      {/if}
    </article>
  {:else}
    <p class="vazio secundario">Nenhum comentário ainda. Registre o que foi feito, com prints e documentos no meio do texto.</p>
  {/each}

  <EditorComentario
    bind:this={editorNovo}
    bind:texto={novoTexto}
    {atividadeId}
    onsalvar={salvarNovo}
    {onanexos}
    onfoco={() => (alvo = "novo")}
    {onerro}
  />
</div>

<style>
  .linha-tempo {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  article {
    border-left: 2px solid var(--borda);
    padding: 0 0 0 12px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    min-height: 24px;
  }

  .acoes {
    display: flex;
    opacity: 0;
    transition: opacity 0.15s;
  }

  article:hover .acoes {
    opacity: 1;
  }

  .acoes button {
    font-size: 12px;
    padding: 2px 6px;
  }

  .corpo {
    line-height: 1.55;
    user-select: text;
    overflow-wrap: anywhere;
  }

  .corpo :global(p) {
    margin: 0 0 6px;
  }

  .corpo :global(p:last-child) {
    margin-bottom: 0;
  }

  .corpo :global(ul),
  .corpo :global(ol) {
    margin: 4px 0;
    padding-left: 20px;
  }

  .corpo :global(code) {
    background: var(--superficie-2);
    border-radius: 4px;
    padding: 1px 4px;
    font-size: 12px;
  }

  .corpo :global(pre) {
    background: var(--superficie-2);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
  }

  .corpo :global(pre code) {
    background: none;
    padding: 0;
  }

  .corpo :global(a) {
    color: var(--destaque);
  }

  .corpo :global(.anexo-img) {
    display: block;
    max-width: 100%;
    max-height: 180px;
    margin: 6px 0;
    border-radius: 6px;
    border: 1px solid var(--borda);
    cursor: zoom-in;
  }

  .corpo :global(.anexo-chip) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    font-size: 12px;
    vertical-align: baseline;
  }

  .corpo :global(.anexo-chip b) {
    color: var(--perigo);
    font-size: 10px;
  }

  .corpo :global(.anexo-faltando) {
    font-size: 12px;
    color: var(--texto-2);
    font-style: italic;
  }

  .vazio {
    font-size: 12px;
    margin: 0;
  }
</style>
