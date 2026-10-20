<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import { api } from "./api";
  import { estado, type AtividadeItem } from "./estado.svelte";
  import IconeTipo from "./IconeTipo.svelte";
  import { DIA_SEG, fmtRelogio } from "./tempo";

  const PARAR = -1;

  let busca = $state("");
  let selecionado = $state(0);
  let campo: HTMLInputElement;
  /** atividade_id -> último início, para ordenar pelas mais recentes */
  let recentes = $state(new Map<number, number>());

  const normalizar = (s: string) =>
    s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

  const itens = $derived.by(() => {
    const termos = normalizar(busca).split(/\s+/).filter(Boolean);
    const lista: (AtividadeItem & { parar?: boolean })[] = estado.atividades
      .filter((a) => a.id !== estado.rodando?.atividade_id || termos.length > 0)
      .filter((a) => {
        const texto = normalizar(`${a.projeto} ${a.nome}`);
        return termos.every((t) => texto.includes(t));
      })
      .sort((a, b) => (recentes.get(b.id) ?? 0) - (recentes.get(a.id) ?? 0) || a.nome.localeCompare(b.nome));
    if (estado.rodando && termos.length === 0) {
      lista.unshift({
        id: PARAR,
        nome: estado.rodando.atividade,
        projeto: estado.rodando.projeto,
        cor: estado.rodando.cor,
        status: "em_andamento",
        tipo_id: null,
        parar: true,
      });
    }
    return lista;
  });

  /** Atividade concluída aguardando o segundo Enter para confirmar. */
  let confirmando = $state<number | null>(null);

  $effect(() => {
    busca;
    selecionado = 0;
  });

  $effect(() => {
    selecionado;
    confirmando = null;
  });

  async function preparar() {
    busca = "";
    selecionado = 0;
    await Promise.all([estado.recarregarProjetos(), estado.recarregarTimer()]);
    const fim = estado.agora;
    const regs = await api.listarRegistros(fim - 14 * DIA_SEG, fim);
    recentes = new Map(regs.map((r) => [r.atividade_id, r.inicio]));
    await tick();
    campo?.focus();
  }

  onMount(() => {
    preparar();
    const desligar = listen("seletor-abrir", preparar);
    return () => desligar.then((f) => f());
  });

  async function escolher(i: number) {
    const item = itens[i];
    if (!item) return;
    if (!item.parar && item.status === "concluida" && confirmando !== item.id) {
      confirmando = item.id;
      return;
    }
    if (item.parar) await api.parar();
    else await api.iniciar(item.id);
    api.fecharSeletor();
  }

  function tecla(e: KeyboardEvent) {
    if (e.key === "Escape") api.fecharSeletor();
    else if (e.key === "ArrowDown") {
      e.preventDefault();
      selecionado = Math.min(selecionado + 1, itens.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selecionado = Math.max(selecionado - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      escolher(selecionado);
    }
  }

  $effect(() => {
    document.querySelector(`[data-indice="${selecionado}"]`)?.scrollIntoView({ block: "nearest" });
  });
</script>

<svelte:window onblur={() => api.fecharSeletor()} onkeydown={tecla} />

<div class="seletor">
  <input bind:this={campo} bind:value={busca} placeholder="Buscar atividade ou projeto..." spellcheck="false" />
  <ul>
    {#each itens as item, i (item.id)}
      <li data-indice={i}>
        <button class:ativo={i === selecionado} onmouseenter={() => (selecionado = i)} onclick={() => escolher(i)}>
          <span class="ponto" style:background={item.cor}></span>
          {#if item.parar}
            <span class="nome"><strong>Parar</strong> {item.nome} <span class="secundario">· {item.projeto}</span></span>
            <span class="num secundario">{fmtRelogio(estado.agora - (estado.rodando?.inicio ?? 0))}</span>
          {:else}
            <IconeTipo tipoId={item.tipo_id} />
            <span class="nome">{item.nome} <span class="secundario">· {item.projeto}</span></span>
            {#if confirmando === item.id}
              <span class="aviso">Concluída. Enter de novo para reabrir e iniciar</span>
            {:else if item.status === "concluida"}
              <span class="selo">Concluída</span>
            {/if}
          {/if}
        </button>
      </li>
    {:else}
      <li class="vazio secundario">
        {estado.atividades.length === 0 ? "Crie atividades na aba Projetos." : "Nenhuma atividade encontrada."}
      </li>
    {/each}
  </ul>
  <footer class="secundario">↑↓ navegar · Enter iniciar · Esc fechar</footer>
</div>

<style>
  .seletor {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--superficie);
    border: 1px solid var(--borda);
  }

  input {
    margin: 12px;
    padding: 10px 12px;
    font-size: 16px;
  }

  ul {
    flex: 1;
    overflow: auto;
    list-style: none;
    margin: 0;
    padding: 0 8px;
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    background: none;
    border: none;
    padding: 9px 10px;
  }

  li button.ativo {
    background: var(--superficie-2);
  }

  .nome {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .selo {
    font-size: 11px;
    color: var(--ok);
  }

  .aviso {
    font-size: 12px;
    color: var(--perigo);
    white-space: nowrap;
  }

  .vazio {
    padding: 20px;
    text-align: center;
  }

  footer {
    font-size: 12px;
    padding: 8px 14px;
    border-top: 1px solid var(--borda);
  }
</style>
