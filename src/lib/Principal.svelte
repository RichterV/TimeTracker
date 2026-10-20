<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { api, type Ociosidade } from "./api";
  import { estado } from "./estado.svelte";
  import { fmtRelogio, inicioDia } from "./tempo";
  import Hoje from "./Hoje.svelte";
  import Semana from "./Semana.svelte";
  import Mes from "./Mes.svelte";
  import Tarefas from "./Tarefas.svelte";
  import Projetos from "./Projetos.svelte";
  import Configuracoes from "./Configuracoes.svelte";
  import OcioModal from "./OcioModal.svelte";
  import AtividadePainel from "./AtividadePainel.svelte";
  import Confirmacao from "./Confirmacao.svelte";
  import IconeTipo from "./IconeTipo.svelte";

  type Aba = "hoje" | "semana" | "mes" | "tarefas" | "projetos" | "config";
  const abas: { id: Aba; nome: string }[] = [
    { id: "hoje", nome: "Dia" },
    { id: "semana", nome: "Semana" },
    { id: "mes", nome: "Mês" },
    { id: "tarefas", nome: "Tarefas" },
    { id: "projetos", nome: "Projetos" },
    { id: "config", nome: "Configurações" },
  ];

  let aba = $state<Aba>("hoje");
  let dia = $state(inicioDia(new Date()));
  let ocio = $state<Ociosidade | null>(null);

  onMount(() => {
    api.ociosidadePendente().then((o) => (ocio = o));
    const desligar = listen<Ociosidade | null>("ociosidade", (ev) => (ocio = ev.payload));
    return () => desligar.then((f) => f());
  });

  function abrirDia(d: Date) {
    dia = d;
    aba = "hoje";
  }
</script>

<div class="app">
  <header>
    <div class="timer">
      {#if estado.rodando}
        <span class="ponto" style:background={estado.rodando.cor}></span>
        <div class="desc">
          <button class="link" onclick={() => (estado.atividadeAberta = estado.rodando!.atividade_id)}>
            <IconeTipo atividadeId={estado.rodando.atividade_id} tamanho={16} />
            <strong>{estado.rodando.atividade}</strong>
          </button>
          <span class="secundario">{estado.rodando.projeto}</span>
        </div>
        <span class="relogio num">{fmtRelogio(estado.agora - estado.rodando.inicio)}</span>
        <button onclick={() => api.mostrarSeletor()}>Trocar</button>
        <button class="primario" onclick={() => api.parar()}>Parar</button>
      {:else}
        <span class="ponto" style:background="var(--borda)"></span>
        <div class="desc secundario">Nenhum timer rodando</div>
        <button class="primario" onclick={() => api.mostrarSeletor()}>
          Iniciar atividade <span class="atalho">{estado.config?.atalho}</span>
        </button>
      {/if}
    </div>
    <nav>
      {#each abas as a}
        <button class:ativa={aba === a.id} onclick={() => (aba = a.id)}>{a.nome}</button>
      {/each}
    </nav>
  </header>

  <main>
    {#if aba === "hoje"}
      <Hoje bind:dia />
    {:else if aba === "semana"}
      <Semana onabrirdia={abrirDia} />
    {:else if aba === "mes"}
      <Mes onabrirdia={abrirDia} />
    {:else if aba === "tarefas"}
      <Tarefas />
    {:else if aba === "projetos"}
      <Projetos />
    {:else}
      <Configuracoes />
    {/if}
  </main>
</div>

{#if estado.atividadeAberta !== null}
  {#key estado.atividadeAberta}
    <AtividadePainel id={estado.atividadeAberta} />
  {/key}
{/if}

{#if ocio}
  <OcioModal {ocio} />
{/if}

{#if estado.confirmacao}
  <Confirmacao />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    background: var(--superficie);
    border-bottom: 1px solid var(--borda);
    padding: 12px 20px 0;
  }

  .timer {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
  }

  .timer .ponto {
    width: 12px;
    height: 12px;
  }

  .desc {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .desc strong {
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .relogio {
    font-size: 26px;
    font-weight: 300;
    margin-right: 8px;
  }

  .atalho {
    opacity: 0.75;
    font-size: 12px;
    margin-left: 6px;
  }

  nav {
    display: flex;
    gap: 4px;
    margin-top: 10px;
  }

  nav button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 8px 12px;
    color: var(--texto-2);
  }

  nav button.ativa {
    color: var(--texto);
    border-bottom-color: var(--destaque);
  }

  main {
    flex: 1;
    overflow: auto;
    padding: 20px;
  }
</style>
