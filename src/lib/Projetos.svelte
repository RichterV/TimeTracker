<script lang="ts">
  import { api, type Projeto } from "./api";
  import { estado, projetoInativo } from "./estado.svelte";
  import { infoStatus } from "./status";
  import IconeTipo from "./IconeTipo.svelte";
  import SeletorTipo from "./SeletorTipo.svelte";

  const CORES = ["#3b6fe0", "#2f9e5b", "#d1453b", "#e0912f", "#8e5bd6", "#1fa2a8", "#c94f8a", "#6b7380"];

  let novoProjeto = $state("");
  let novaAtividade = $state<Record<number, string>>({});
  let tipoNovaAtividade = $state<Record<number, number | null>>({});
  let mostrarArquivados = $state(false);
  let erro = $state("");

  const projetos = $derived(estado.projetos.filter((p) => mostrarArquivados || !projetoInativo(p)));
  const atividadesDe = (p: Projeto) => p.atividades.filter((a) => mostrarArquivados || a.status !== "finalizada");

  async function executar(f: () => Promise<unknown>) {
    erro = "";
    try {
      await f();
    } catch (e) {
      erro = String(e);
    }
    await estado.recarregarProjetos();
  }

  function criarProjeto(e: SubmitEvent) {
    e.preventDefault();
    const nome = novoProjeto.trim();
    if (!nome) return;
    novoProjeto = "";
    executar(() => api.criarProjeto(nome, CORES[estado.projetos.length % CORES.length]));
  }

  function criarAtividade(e: SubmitEvent, p: Projeto) {
    e.preventDefault();
    const nome = (novaAtividade[p.id] ?? "").trim();
    const tipo = tipoNovaAtividade[p.id] ?? estado.tipoPadrao;
    if (!nome || tipo === null) return;
    novaAtividade[p.id] = "";
    executar(() => api.criarAtividade(p.id, nome, tipo));
  }

  function renomearProjeto(p: Projeto, nome: string) {
    nome = nome.trim();
    if (nome && nome !== p.nome) executar(() => api.editarProjeto(p.id, nome, p.cor, p.arquivado));
  }
</script>

<div class="topo">
  <form onsubmit={criarProjeto}>
    <input bind:value={novoProjeto} placeholder="Novo projeto" />
    <button class="primario" type="submit">Adicionar</button>
  </form>
  <label class="secundario">
    <input type="checkbox" bind:checked={mostrarArquivados} /> Mostrar arquivados e finalizados
  </label>
</div>

{#if erro}<p class="erro">{erro}</p>{/if}

{#each projetos as p (p.id)}
  <section class="cartao projeto" class:arquivado={projetoInativo(p)}>
    <header>
      <input
        type="color"
        value={p.cor}
        title="Cor do projeto"
        onchange={(e) => executar(() => api.editarProjeto(p.id, p.nome, e.currentTarget.value, p.arquivado))}
      />
      <input class="nome" value={p.nome} onchange={(e) => renomearProjeto(p, e.currentTarget.value)} />
      {#if p.finalizado}
        <span class="selo" style:--cor="var(--ok)">Finalizado</span>
        <button class="icone" onclick={() => executar(() => api.finalizarProjeto(p.id, false))}>Reabrir</button>
      {:else if p.arquivado}
        <span class="selo" style:--cor="var(--texto-2)">Arquivado</span>
        <button class="icone" onclick={() => executar(() => api.editarProjeto(p.id, p.nome, p.cor, false))}>Desarquivar</button>
      {:else}
        <button class="icone" onclick={() => executar(() => api.finalizarProjeto(p.id, true))}>Finalizar</button>
        <button class="icone" onclick={() => executar(() => api.editarProjeto(p.id, p.nome, p.cor, true))}>Arquivar</button>
      {/if}
      <button class="icone perigo" title="Excluir projeto" onclick={async () => (await estado.confirmar(`Excluir o projeto "${p.nome}", suas atividades e anexos?`, "Excluir")) && executar(() => api.excluirProjeto(p.id))}>✕</button>
    </header>
    <ul>
      {#each atividadesDe(p) as a (a.id)}
        {@const s = infoStatus(a.status)}
        <li class:arquivado={a.status === "finalizada"}>
          <IconeTipo tipoId={a.tipo_id} />
          <button class="link nome-atividade" onclick={() => (estado.atividadeAberta = a.id)} title="Abrir detalhes">
            {a.nome}
          </button>
          <span class="selo" style:--cor={s.cor}>{s.nome}</span>
          <button class="icone" onclick={() => estado.iniciarAtividade(a.id)} title="Iniciar timer" disabled={projetoInativo(p)}>▶</button>
        </li>
      {/each}
      <li>
        <form onsubmit={(e) => criarAtividade(e, p)}>
          <SeletorTipo compacto bind:value={() => tipoNovaAtividade[p.id] ?? estado.tipoPadrao, (v) => (tipoNovaAtividade[p.id] = v)} />
          <input bind:value={novaAtividade[p.id]} placeholder="+ Nova atividade" class="nome" />
        </form>
      </li>
    </ul>
  </section>
{:else}
  <p class="vazio secundario">Nenhum projeto ainda. Crie o primeiro acima.</p>
{/each}

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .topo form {
    display: flex;
    gap: 8px;
  }

  .topo form input {
    width: 260px;
  }

  .topo label {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .projeto {
    margin-bottom: 12px;
    padding: 10px 12px;
  }

  .arquivado {
    opacity: 0.55;
  }

  header,
  li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  header .nome {
    font-weight: 600;
    font-size: 15px;
  }

  input[type="color"] {
    width: 28px;
    height: 28px;
    padding: 2px;
    cursor: pointer;
  }

  .nome {
    flex: 1;
    border-color: transparent;
    background: transparent;
  }

  .nome:hover {
    border-color: var(--borda);
  }

  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0 0 0 34px;
  }

  li {
    min-height: 34px;
  }

  .nome-atividade {
    flex: 1;
    padding: 4px 9px;
  }

  .selo {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    color: var(--cor);
    background: color-mix(in srgb, var(--cor) 14%, transparent);
    white-space: nowrap;
  }

  li form {
    flex: 1;
    display: flex;
    gap: 6px;
  }

  .vazio {
    text-align: center;
    padding: 40px 0;
  }
</style>
