<script lang="ts">
  import { untrack } from "svelte";
  import { api, type Registro } from "./api";
  import { estado } from "./estado.svelte";
  import Modal from "./Modal.svelte";
  import SeletorAtividade from "./SeletorAtividade.svelte";
  import { DIA_SEG, deInputs, paraInputData, paraInputHora, seg } from "./tempo";

  let {
    registro,
    dia,
    intervalo,
    onfechar,
  }: {
    registro: Registro | null;
    dia: Date;
    /** início/fim sugeridos para um registro novo (ex.: arrastado no calendário) */
    intervalo?: { inicio: number; fim: number };
    onfechar: () => void;
  } = $props();

  // o formulário parte dos valores de abertura do modal
  const r = untrack(() => registro);
  const sugerido = untrack(() => intervalo);
  const rodando = r !== null && r.fim === null;
  const padraoInicio =
    r?.inicio ?? sugerido?.inicio ?? Math.min(seg(untrack(() => dia)) + 9 * 3600, estado.agora - 3600);
  const padraoFim = r ? r.fim : (sugerido?.fim ?? padraoInicio + 3600);

  let atividadeId = $state<number | null>(r?.atividade_id ?? estado.atividades[0]?.id ?? null);
  let data = $state(paraInputData(new Date(padraoInicio * 1000)));
  let inicio = $state(paraInputHora(padraoInicio));
  let fim = $state(padraoFim ? paraInputHora(padraoFim) : "");
  let nota = $state(r?.nota ?? "");
  let erro = $state("");

  async function salvar(e: SubmitEvent) {
    e.preventDefault();
    if (atividadeId === null) return (erro = "Escolha a atividade.");
    const ini = deInputs(data, inicio);
    let f: number | null = null;
    if (!rodando) {
      f = deInputs(data, fim);
      if (f <= ini) f += DIA_SEG; // passou da meia-noite
    }
    try {
      await api.salvarRegistro({ id: r?.id ?? null, atividade_id: atividadeId, inicio: ini, fim: f, nota: nota.trim() });
      onfechar();
    } catch (x) {
      erro = String(x);
    }
  }
</script>

<Modal titulo={registro ? "Editar registro" : "Lançar tempo"} {onfechar}>
  <form onsubmit={salvar}>
    <label>
      Atividade
      <SeletorAtividade bind:value={atividadeId} />
    </label>
    <div class="linha">
      <label>
        Data
        <input type="date" bind:value={data} required />
      </label>
      <label>
        Início
        <input type="time" bind:value={inicio} required />
      </label>
      <label>
        Fim
        {#if rodando}
          <input value="rodando" disabled />
        {:else}
          <input type="time" bind:value={fim} required />
        {/if}
      </label>
    </div>
    <label>
      Nota
      <textarea rows="3" bind:value={nota} placeholder="O que foi feito (opcional)"></textarea>
    </label>
    {#if erro}<p class="erro">{erro}</p>{/if}
    <div class="acoes">
      <button type="button" onclick={onfechar}>Cancelar</button>
      <button type="submit" class="primario">Salvar</button>
    </div>
  </form>
</Modal>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--texto-2);
  }

  label :global(select),
  label input,
  label textarea {
    font-size: 14px;
    color: var(--texto);
  }

  .linha {
    display: grid;
    grid-template-columns: 1.4fr 1fr 1fr;
    gap: 8px;
  }

  textarea {
    resize: vertical;
  }

  .acoes {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .erro {
    margin: 0;
  }
</style>
