<script lang="ts">
  import { untrack } from "svelte";
  import { api } from "./api";
  import { corParaNovoProjeto } from "./cores";
  import { estado, projetoInativo } from "./estado.svelte";
  import SeletorTipo from "./SeletorTipo.svelte";

  const NOVO_PROJETO = -1;

  let {
    nomeInicial = "",
    oncriada,
    oncancelar,
  }: {
    nomeInicial?: string;
    /** chamado com o id criado e se o timer deve iniciar */
    oncriada: (id: number, iniciar: boolean) => void;
    oncancelar: () => void;
  } = $props();

  const lerPref = (k: string) => {
    try {
      return Number(localStorage.getItem(k)) || null;
    } catch {
      return null;
    }
  };

  const projetos = $derived(estado.projetos.filter((p) => !projetoInativo(p)));

  let nome = $state(untrack(() => nomeInicial));
  let projetoId = $state<number | null>(
    untrack(() => {
      const ultimo = lerPref("seletor.projeto");
      return projetos.some((p) => p.id === ultimo) ? ultimo : (projetos[0]?.id ?? NOVO_PROJETO);
    }),
  );
  let novoProjeto = $state("");
  let tipoId = $state<number | null>(untrack(() => estado.tipoPadrao));
  let salvando = $state(false);
  let erro = $state("");

  function focar(no: HTMLInputElement) {
    no.focus();
    no.select();
  }

  async function criar(iniciar: boolean) {
    if (salvando) return;
    if (!nome.trim()) return (erro = "Informe o nome da atividade.");
    if (projetoId === NOVO_PROJETO && !novoProjeto.trim()) return (erro = "Informe o nome do projeto.");
    if (projetoId === null || tipoId === null) return (erro = "Escolha projeto e tipo.");
    salvando = true;
    erro = "";
    try {
      let pid = projetoId;
      if (pid === NOVO_PROJETO) pid = await api.criarProjeto(novoProjeto.trim(), corParaNovoProjeto(estado.projetos.length));
      const id = await api.criarAtividade(pid, nome.trim(), tipoId);
      try {
        localStorage.setItem("seletor.projeto", String(pid));
      } catch {}
      await estado.recarregarProjetos();
      oncriada(id, iniciar);
    } catch (e) {
      erro = String(e);
    } finally {
      salvando = false;
    }
  }

  function tecla(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      criar(!e.shiftKey);
    } else if (e.key === "Escape") {
      e.preventDefault();
      oncancelar();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="nova" onkeydown={tecla}>
  <h3>Nova atividade</h3>
  <input class="nome" bind:value={nome} placeholder="Nome da atividade" use:focar />

  <div class="linha">
    <label>
      <span class="secundario">Projeto</span>
      <select bind:value={projetoId}>
        {#each projetos as p (p.id)}
          <option value={p.id}>{p.nome}</option>
        {/each}
        <option value={NOVO_PROJETO}>＋ Novo projeto...</option>
      </select>
    </label>
    <label class="tipo">
      <span class="secundario">Tipo</span>
      <SeletorTipo bind:value={tipoId} />
    </label>
  </div>

  {#if projetoId === NOVO_PROJETO}
    <input bind:value={novoProjeto} placeholder="Nome do novo projeto" />
  {/if}

  {#if erro}<p class="erro">{erro}</p>{/if}

  <div class="acoes">
    <button type="button" onclick={oncancelar}>Voltar</button>
    <button type="button" onclick={() => criar(false)} disabled={salvando}>Só criar</button>
    <button type="button" class="primario" onclick={() => criar(true)} disabled={salvando}>Criar e iniciar</button>
  </div>
</div>

<style>
  .nova {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 14px 12px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .nome {
    font-size: 15px;
    padding: 8px 10px;
  }

  .linha {
    display: flex;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    flex: 1;
    min-width: 0;
  }

  label select {
    font-size: 14px;
  }

  .tipo {
    flex: 0 0 auto;
  }

  .acoes {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .erro {
    margin: 0;
    font-size: 12px;
  }
</style>
