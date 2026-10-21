<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { api, type Config } from "./api";
  import { estado } from "./estado.svelte";
  import { DIAS_CURTOS } from "./tempo";
  import Tipos from "./Tipos.svelte";
  import Folgas from "./Folgas.svelte";

  let cfg = $state<Config>($state.snapshot(estado.config!));
  let metaHoras = $state(cfg.meta_diaria_min / 60);
  let pasta = $state("");
  let mensagem = $state("");
  let erro = $state("");

  onMount(() => {
    api.pastaDados().then((c) => (pasta = c));
  });

  async function mudarPasta() {
    const escolhida = await open({ directory: true, title: "Pasta para o banco e os anexos do 8Pace" });
    if (!escolhida || escolhida === pasta) return;
    erro = "";
    try {
      const temBanco = await api.pastaTemBanco(escolhida);
      const ok = temBanco
        ? await estado.confirmar(
            `A pasta "${escolhida}" já tem dados do 8Pace. Passar a usar esses dados? Os dados atuais continuam na pasta antiga. O 8Pace vai reiniciar.`,
            "Usar dados da pasta",
          )
        : await estado.confirmar(
            `Copiar o banco e os anexos para "${escolhida}" e passar a usar essa pasta? Os arquivos antigos não são apagados. O 8Pace vai reiniciar.`,
            "Copiar e usar",
          );
      if (ok) await api.mudarPastaDados(escolhida, !temBanco);
    } catch (x) {
      erro = String(x);
    }
  }

  function alternarDia(d: number) {
    cfg.dias_uteis = cfg.dias_uteis.includes(d)
      ? cfg.dias_uteis.filter((x) => x !== d)
      : [...cfg.dias_uteis, d].sort();
  }

  const FACULTATIVOS = [
    { id: "carnaval", nome: "Carnaval (segunda e terça)" },
    { id: "cinzas", nome: "Quarta de Cinzas (até 14h, meta pela metade)" },
    { id: "corpus", nome: "Corpus Christi" },
  ];

  function alternarFacultativo(id: string) {
    cfg.facultativos = cfg.facultativos.includes(id)
      ? cfg.facultativos.filter((x) => x !== id)
      : [...cfg.facultativos, id];
  }

  async function salvar(e: SubmitEvent) {
    e.preventDefault();
    erro = mensagem = "";
    cfg.meta_diaria_min = Math.round(metaHoras * 60);
    try {
      await api.salvarConfig($state.snapshot(cfg));
      estado.config = await api.obterConfig();
      await estado.recarregarEspeciais();
      mensagem = "Configurações salvas.";
    } catch (x) {
      erro = String(x);
    }
  }
</script>

<form onsubmit={salvar}>
  <section class="cartao">
    <h2>Jornada</h2>
    <div class="campo">
      <span>Meta diária (horas)</span>
      <input type="number" min="0" max="24" step="0.25" bind:value={metaHoras} />
    </div>
    <div class="campo">
      <span>Dias úteis</span>
      <div class="dias">
        {#each DIAS_CURTOS as nome, i}
          <button type="button" class:ativo={cfg.dias_uteis.includes(i + 1)} onclick={() => alternarDia(i + 1)}>
            {nome}
          </button>
        {/each}
      </div>
    </div>
    <div class="campo topo">
      <span>Expediente</span>
      <div class="intervalos">
        {#each cfg.expediente as intervalo, i}
          <div class="intervalo">
            <input type="time" bind:value={intervalo.inicio} required />
            <span class="secundario">até</span>
            <input type="time" bind:value={intervalo.fim} required />
            <button type="button" class="icone" onclick={() => cfg.expediente.splice(i, 1)}>✕</button>
          </div>
        {/each}
        <button type="button" class="icone adicionar" onclick={() => cfg.expediente.push({ inicio: "08:00", fim: "12:00" })}>
          + Intervalo
        </button>
      </div>
    </div>
  </section>

  <section class="cartao">
    <h2>Feriados</h2>
    <p class="dica secundario">Feriados nacionais são marcados automaticamente. Pontos facultativos considerados:</p>
    {#each FACULTATIVOS as f}
      <label class="campo opcao">
        <span>{f.nome}</span>
        <input type="checkbox" checked={cfg.facultativos.includes(f.id)} onchange={() => alternarFacultativo(f.id)} />
      </label>
    {/each}
  </section>

  <section class="cartao">
    <h2>Avisos</h2>
    <div class="campo">
      <span>Ociosidade (minutos sem teclado e mouse)</span>
      <input type="number" min="0" max="240" bind:value={cfg.ociosidade_min} />
    </div>
    <p class="dica secundario">Use 0 para desligar a detecção de ociosidade.</p>
    <div class="campo">
      <span>Lembrar quando não houver timer no expediente</span>
      <input type="checkbox" bind:checked={cfg.lembrete_ativo} />
    </div>
    <div class="campo">
      <span>Repetir lembrete a cada (minutos)</span>
      <input type="number" min="1" max="240" bind:value={cfg.lembrete_intervalo_min} disabled={!cfg.lembrete_ativo} />
    </div>
  </section>

  <section class="cartao">
    <h2>Sistema</h2>
    <div class="campo">
      <span>Atalho global do seletor</span>
      <input bind:value={cfg.atalho} placeholder="Ctrl+Alt+Space" />
    </div>
    <div class="campo">
      <span>Iniciar com o Windows (minimizado)</span>
      <input type="checkbox" bind:checked={cfg.iniciar_com_windows} />
    </div>
  </section>

  <section class="cartao">
    <h2>Dados</h2>
    <p class="dica secundario">Pasta com o banco (8pace.db) e os anexos. Pode ser uma pasta de backup, como o OneDrive.</p>
    <div class="campo">
      <code>{pasta}</code>
      <div class="botoes">
        <button type="button" onclick={() => api.abrirPastaDados()}>Abrir</button>
        <button type="button" onclick={mudarPasta}>Alterar...</button>
      </div>
    </div>
  </section>

  <div class="rodape">
    {#if erro}<span class="erro">{erro}</span>{/if}
    {#if mensagem}<span class="secundario">{mensagem}</span>{/if}
    <button type="submit" class="primario">Salvar</button>
  </div>
</form>

<div class="tipos"><Tipos /></div>
<div class="tipos"><Folgas /></div>

<style>
  .tipos {
    max-width: 640px;
    margin-top: 14px;
  }

  form {
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  section {
    padding: 14px 16px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 15px;
    font-weight: 600;
  }

  .campo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 6px 0;
  }

  .campo.topo {
    align-items: flex-start;
  }

  .campo input[type="number"] {
    width: 90px;
  }

  .dias {
    display: flex;
    gap: 4px;
  }

  .dias button {
    padding: 4px 8px;
  }

  .dias button.ativo {
    background: var(--destaque);
    border-color: var(--destaque);
    color: var(--destaque-texto);
  }

  .intervalos {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-end;
  }

  .intervalo {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dica {
    margin: -4px 0 4px;
    font-size: 12px;
  }

  code {
    font-size: 12px;
    color: var(--texto-2);
    user-select: text;
    word-break: break-all;
  }

  .botoes {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .rodape {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
  }
</style>
