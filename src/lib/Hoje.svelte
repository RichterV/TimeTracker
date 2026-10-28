<script lang="ts">
  import { api, type Registro } from "./api";
  import { estado } from "./estado.svelte";
  import RegistroModal from "./RegistroModal.svelte";
  import IconeTipo from "./IconeTipo.svelte";
  import SeloEspecial from "./SeloEspecial.svelte";
  import {
    DIA_SEG,
    duracaoEm,
    fmtDataLonga,
    fmtDur,
    fmtHora,
    inicioDia,
    mesmoDia,
    seg,
    somarDias,
  } from "./tempo";

  let { dia = $bindable() }: { dia: Date } = $props();

  let registros = $state<Registro[]>([]);
  let editando = $state<Registro | null | undefined>(undefined); // undefined = fechado, null = novo

  const de = $derived(seg(dia));
  const ate = $derived(de + DIA_SEG);
  const ehHoje = $derived(mesmoDia(dia, new Date(estado.agora * 1000)));

  $effect(() => {
    estado.versao;
    api.listarRegistros(de, ate).then((r) => (registros = r));
  });

  const total = $derived(registros.reduce((s, r) => s + duracaoEm(r, de, ate, estado.agora), 0));
  const meta = $derived(estado.metaDoDia(dia));
  const especial = $derived(estado.especial(dia));

  $effect(() => {
    estado.garantirAnos(dia.getFullYear());
  });

  const porAtividade = $derived.by(() => {
    const m = new Map<number, { nome: string; projeto: string; cor: string; seg: number }>();
    for (const r of registros) {
      const item = m.get(r.atividade_id) ?? { nome: r.atividade, projeto: r.projeto, cor: r.cor, seg: 0 };
      item.seg += duracaoEm(r, de, ate, estado.agora);
      m.set(r.atividade_id, item);
    }
    return [...m.values()].sort((a, b) => b.seg - a.seg);
  });

</script>

<div class="topo">
  <div class="navegacao">
    <button class="icone" onclick={() => (dia = somarDias(dia, -1))} title="Dia anterior">‹</button>
    <h1>{ehHoje ? "Hoje" : fmtDataLonga(dia)}</h1>
    <button class="icone" onclick={() => (dia = somarDias(dia, 1))} title="Próximo dia">›</button>
    {#if !ehHoje}
      <button class="icone texto" onclick={() => (dia = inicioDia(new Date()))}>Voltar para hoje</button>
    {/if}
  </div>
  <button onclick={() => (editando = null)} disabled={estado.atividades.length === 0}>+ Lançar tempo</button>
</div>

{#if ehHoje || especial}
  <p class="secundario subtitulo">
    {#if ehHoje}{fmtDataLonga(dia)}{/if}
    <SeloEspecial dia={especial} />
  </p>
{/if}

<section class="cartao resumo">
  <div class="totais">
    <span class="num grande">{fmtDur(total)}</span>
    {#if meta > 0}
      <span class="secundario">
        de {fmtDur(meta)}
        {#if total < meta}· faltam {fmtDur(meta - total)}{:else}· meta atingida{/if}
      </span>
    {/if}
  </div>
  {#if meta > 0}
    <div class="barra"><div style:width="{Math.min(100, (total / meta) * 100)}%" class:ok={total >= meta}></div></div>
  {/if}
  {#if porAtividade.length > 0}
    <ul class="atividades">
      {#each porAtividade as a}
        <li>
          <span class="ponto" style:background={a.cor}></span>
          <span class="nome">{a.nome} <span class="secundario">· {a.projeto}</span></span>
          <span class="num">{fmtDur(a.seg)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if estado.projetos.length === 0}
  <p class="vazio secundario">Comece criando um projeto e suas atividades na aba <strong>Projetos</strong>.</p>
{:else if registros.length === 0}
  <p class="vazio secundario">Nenhum registro neste dia.</p>
{:else}
  <ul class="registros cartao">
    {#each registros as r (r.id)}
      <li class:rodando={r.fim === null}>
        <span class="ponto" style:background={r.cor}></span>
        <div class="info">
          <div>
            <IconeTipo atividadeId={r.atividade_id} />
            <button class="link" onclick={() => (estado.atividadeAberta = r.atividade_id)}>{r.atividade}</button>
            <span class="secundario">· {r.projeto}</span>
          </div>
          {#if r.nota}<div class="nota secundario">{r.nota}</div>{/if}
        </div>
        <span class="horario num secundario">{fmtHora(r.inicio)} – {r.fim ? fmtHora(r.fim) : "agora"}</span>
        <span class="duracao num">{fmtDur((r.fim ?? estado.agora) - r.inicio)}</span>
        <div class="acoes">
          {#if r.fim !== null}
            <button class="icone" title="Continuar esta atividade" onclick={() => estado.iniciarAtividade(r.atividade_id)}>▶</button>
          {/if}
          <button class="icone" title="Editar" onclick={() => (editando = r)}>✎</button>
          <button class="icone" title="Excluir registro" onclick={() => estado.excluirRegistro(r)}>✕</button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

{#if editando !== undefined}
  <RegistroModal registro={editando} {dia} onfechar={() => (editando = undefined)} />
{/if}

<style>
  .topo {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .navegacao {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .navegacao button {
    font-size: 18px;
  }

  .navegacao button.texto {
    font-size: 13px;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    text-transform: capitalize;
  }

  .subtitulo {
    margin: 2px 0 0 36px;
    text-transform: capitalize;
  }

  .resumo {
    margin: 16px 0;
    padding: 16px;
  }

  .totais {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .grande {
    font-size: 28px;
    font-weight: 600;
  }

  .barra {
    height: 6px;
    background: var(--superficie-2);
    border-radius: 3px;
    margin-top: 10px;
    overflow: hidden;
  }

  .barra div {
    height: 100%;
    background: var(--destaque);
    transition: width 0.3s;
  }

  .barra div.ok {
    background: var(--ok);
  }

  .atividades {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }

  .atividades li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .atividades .nome {
    flex: 1;
  }

  .vazio {
    text-align: center;
    padding: 40px 0;
  }

  .registros {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .registros li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--borda);
  }

  .registros li:last-child {
    border-bottom: none;
  }

  .registros li.rodando {
    background: color-mix(in srgb, var(--destaque) 8%, transparent);
  }

  .info {
    flex: 1;
    min-width: 0;
  }

  .nota {
    font-size: 12px;
    margin-top: 2px;
    white-space: pre-wrap;
    user-select: text;
  }

  .duracao {
    width: 52px;
    text-align: right;
    font-weight: 600;
  }

  .acoes {
    display: flex;
    gap: 2px;
    min-width: 110px;
    justify-content: flex-end;
  }
</style>
