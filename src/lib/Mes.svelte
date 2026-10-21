<script lang="ts">
  import { api, type Registro } from "./api";
  import { estado } from "./estado.svelte";
  import IconeTipo from "./IconeTipo.svelte";
  import SeloEspecial from "./SeloEspecial.svelte";
  import { COR_ESPECIAL } from "./especiais";
  import { DIAS_CURTOS, duracaoEm, fmtDur, inicioSemana, mesmoDia, seg, somarDias } from "./tempo";

  let { onabrirdia }: { onabrirdia: (d: Date) => void } = $props();

  const primeiroDoMes = (d: Date) => new Date(d.getFullYear(), d.getMonth(), 1);

  let mes = $state(primeiroDoMes(new Date()));
  let registros = $state<Registro[]>([]);

  const proximoMes = $derived(new Date(mes.getFullYear(), mes.getMonth() + 1, 1));
  const de = $derived(seg(mes));
  const ate = $derived(seg(proximoMes));

  $effect(() => {
    estado.versao;
    api.listarRegistros(de, ate).then((r) => (registros = r));
  });

  type Fatia = { cor: string; seg: number };
  type Celula = { data: Date; doMes: boolean; total: number; fatias: Fatia[] };

  /** Semanas (segunda a domingo) que cobrem o mês, com total e fatias por projeto. */
  const semanas = $derived.by(() => {
    const inicio = inicioSemana(mes);
    const lista: Celula[][] = [];
    for (let d = inicio; d < proximoMes; d = somarDias(d, 7)) {
      lista.push(
        Array.from({ length: 7 }, (_, i) => {
          const data = somarDias(d, i);
          const doMes = data.getMonth() === mes.getMonth();
          const a = seg(data);
          const b = seg(somarDias(data, 1));
          const porCor = new Map<string, number>();
          let total = 0;
          if (doMes) {
            for (const r of registros) {
              const s = duracaoEm(r, a, b, estado.agora);
              if (s > 0) {
                total += s;
                porCor.set(r.cor, (porCor.get(r.cor) ?? 0) + s);
              }
            }
          }
          const fatias = [...porCor].map(([cor, s]) => ({ cor, seg: s })).sort((x, y) => y.seg - x.seg);
          return { data, doMes, total, fatias };
        }),
      );
    }
    return lista;
  });

  type ResumoAtividade = { id: number; nome: string; seg: number };
  type ResumoProjeto = { nome: string; cor: string; seg: number; atividades: Map<number, ResumoAtividade> };

  const resumo = $derived.by(() => {
    const projetos = new Map<string, ResumoProjeto>();
    let total = 0;
    for (const r of registros) {
      const s = duracaoEm(r, de, ate, estado.agora);
      if (s <= 0) continue;
      total += s;
      let p = projetos.get(r.projeto);
      if (!p) projetos.set(r.projeto, (p = { nome: r.projeto, cor: r.cor, seg: 0, atividades: new Map() }));
      p.seg += s;
      const a = p.atividades.get(r.atividade_id) ?? { id: r.atividade_id, nome: r.atividade, seg: 0 };
      a.seg += s;
      p.atividades.set(r.atividade_id, a);
    }
    const lista = [...projetos.values()]
      .sort((a, b) => b.seg - a.seg)
      .map((p) => ({ ...p, atividades: [...p.atividades.values()].sort((a, b) => b.seg - a.seg) }));
    return { lista, total };
  });

  /** Dias do mês com meta (meio período conta como meio dia). */
  const metasDoMes = $derived.by(() => {
    const lista: number[] = [];
    for (let d = new Date(mes); d < proximoMes; d = somarDias(d, 1)) lista.push(estado.metaDoDia(d));
    return lista;
  });
  const meta = $derived(metasDoMes.reduce((a, b) => a + b, 0));
  const metaCheia = $derived((estado.config?.meta_diaria_min ?? 0) * 60);
  const diasUteis = $derived(metaCheia > 0 ? Math.round((meta / metaCheia) * 2) / 2 : 0);

  $effect(() => {
    estado.garantirAnos(mes.getFullYear());
  });
  const hoje = $derived(new Date(estado.agora * 1000));
  const titulo = $derived(mes.toLocaleDateString("pt-BR", { month: "long", year: "numeric" }));
  const pct = (s: number, t: number) => (t > 0 ? Math.round((s / t) * 100) : 0);
</script>

<div class="topo">
  <div class="navegacao">
    <button class="icone" onclick={() => (mes = new Date(mes.getFullYear(), mes.getMonth() - 1, 1))} title="Mês anterior">‹</button>
    <h1>{titulo}</h1>
    <button class="icone" onclick={() => (mes = proximoMes)} title="Próximo mês">›</button>
    {#if !mesmoDia(mes, primeiroDoMes(hoje))}
      <button class="icone texto" onclick={() => (mes = primeiroDoMes(new Date()))}>Mês atual</button>
    {/if}
  </div>
  <div class="secundario">
    <strong class="num total">{fmtDur(resumo.total)}</strong>
    {#if meta > 0} de {fmtDur(meta)} ({diasUteis} dias úteis){/if}
  </div>
</div>

<div class="cartao calendario">
  {#each DIAS_CURTOS as nome}
    <div class="cabecalho secundario">{nome}</div>
  {/each}
  {#each semanas as semana}
    {#each semana as c}
      {#if c.doMes}
        {@const especial = estado.especial(c.data)}
        <button
          class="dia"
          class:hoje={mesmoDia(c.data, hoje)}
          class:hachurado={!!especial}
          style:--hachura={especial ? COR_ESPECIAL[especial.tipo] : null}
          onclick={() => onabrirdia(c.data)}
        >
          <span class="numero">{c.data.getDate()}</span>
          <SeloEspecial dia={estado.especial(c.data)} compacto />
          {#if c.total > 0}
            <span class="num horas" class:meta={estado.metaDoDia(c.data) > 0 && c.total >= estado.metaDoDia(c.data)}>{fmtDur(c.total)}</span>
            <span class="fatias">
              {#each c.fatias as f}
                <span style:background={f.cor} style:flex-grow={f.seg}></span>
              {/each}
            </span>
          {/if}
        </button>
      {:else}
        <div class="dia fora"></div>
      {/if}
    {/each}
  {/each}
</div>

<h2>Por projeto</h2>
{#if resumo.lista.length === 0}
  <p class="vazio secundario">Nenhum registro neste mês.</p>
{:else}
  <div class="cartao resumo">
    {#each resumo.lista as p}
      <div class="projeto">
        <div class="linha forte">
          <span class="ponto" style:background={p.cor}></span>
          <span class="nome">{p.nome}</span>
          <span class="barra"><span style:width="{pct(p.seg, resumo.total)}%" style:background={p.cor}></span></span>
          <span class="num pct secundario">{pct(p.seg, resumo.total)}%</span>
          <span class="num dur">{fmtDur(p.seg)}</span>
        </div>
        {#each p.atividades as a}
          <div class="linha atividade">
            <span class="nome"><IconeTipo atividadeId={a.id} /> {a.nome}</span>
            <span class="num pct secundario">{pct(a.seg, resumo.total)}%</span>
            <span class="num dur">{fmtDur(a.seg)}</span>
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/if}

<style>
  .topo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
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

  .total {
    font-size: 20px;
    color: var(--texto);
  }

  .calendario {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px;
    background: var(--borda);
    overflow: hidden;
  }

  .cabecalho {
    background: var(--superficie);
    padding: 8px;
    text-align: center;
    font-size: 12px;
  }

  .dia {
    background-color: var(--superficie);
    border: none;
    border-radius: 0;
    min-height: 72px;
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    text-align: left;
  }

  .dia:hover {
    background-color: var(--superficie-2);
    filter: none;
  }

  .dia.fora {
    background-color: var(--fundo);
  }

  .numero {
    font-size: 12px;
    color: var(--texto-2);
  }

  .hoje .numero {
    color: var(--destaque);
    font-weight: 700;
  }

  .horas {
    font-weight: 600;
    font-size: 15px;
  }

  .horas.meta {
    color: var(--ok);
  }

  .fatias {
    display: flex;
    width: 100%;
    height: 4px;
    border-radius: 2px;
    overflow: hidden;
    margin-top: auto;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    margin: 20px 0 10px;
  }

  .resumo {
    padding: 8px 14px;
  }

  .projeto {
    padding: 6px 0;
    border-bottom: 1px solid var(--borda);
  }

  .projeto:last-child {
    border-bottom: none;
  }

  .linha {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 3px 0;
  }

  .forte {
    font-weight: 600;
  }

  .atividade {
    padding-left: 20px;
  }

  .linha .nome {
    flex: 1;
  }

  .barra {
    width: 160px;
    height: 6px;
    background: var(--superficie-2);
    border-radius: 3px;
    overflow: hidden;
  }

  .barra span {
    display: block;
    height: 100%;
  }

  .pct {
    width: 40px;
    text-align: right;
    font-weight: 400;
  }

  .dur {
    width: 56px;
    text-align: right;
  }

  .vazio {
    text-align: center;
    padding: 30px 0;
  }
</style>
