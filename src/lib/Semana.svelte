<script lang="ts">
  import { api, type Registro } from "./api";
  import { estado } from "./estado.svelte";
  import IconeTipo from "./IconeTipo.svelte";
  import CalendarioSemana from "./CalendarioSemana.svelte";
  import SeloEspecial from "./SeloEspecial.svelte";
  import { COR_ESPECIAL } from "./especiais";
  import {
    DIAS_CURTOS,
    DIA_SEG,
    duracaoEm,
    fmtDataCurta,
    fmtDur,
    inicioSemana,
    mesmoDia,
    seg,
    somarDias,
  } from "./tempo";

  let { onabrirdia }: { onabrirdia: (d: Date) => void } = $props();

  let semana = $state(inicioSemana(new Date()));
  let registros = $state<Registro[]>([]);

  // preferências de visualização ficam só neste computador
  const lerPref = (k: string, padrao: string) => {
    try {
      return localStorage.getItem(k) ?? padrao;
    } catch {
      return padrao;
    }
  };
  let modo = $state<"calendario" | "tabela">(lerPref("semana.modo", "calendario") as "calendario" | "tabela");
  let verFimDeSemana = $state(lerPref("semana.fds", "0") === "1");

  $effect(() => {
    try {
      localStorage.setItem("semana.modo", modo);
      localStorage.setItem("semana.fds", verFimDeSemana ? "1" : "0");
    } catch {}
  });

  const dias = $derived(Array.from({ length: 7 }, (_, i) => somarDias(semana, i)));
  const limites = $derived(dias.map((d) => [seg(d), seg(somarDias(d, 1))] as const));

  $effect(() => {
    estado.versao;
    api.listarRegistros(limites[0][0], limites[6][1]).then((r) => (registros = r));
  });

  type Linha = { id: number; nome: string; dias: number[]; total: number };
  type Grupo = { nome: string; cor: string; linhas: Linha[]; dias: number[]; total: number };

  const grade = $derived.by(() => {
    const grupos = new Map<string, Grupo>();
    const linhas = new Map<number, Linha>();
    const totais = Array(7).fill(0);
    for (const r of registros) {
      let g = grupos.get(r.projeto);
      if (!g) grupos.set(r.projeto, (g = { nome: r.projeto, cor: r.cor, linhas: [], dias: Array(7).fill(0), total: 0 }));
      let l = linhas.get(r.atividade_id);
      if (!l) {
        linhas.set(r.atividade_id, (l = { id: r.atividade_id, nome: r.atividade, dias: Array(7).fill(0), total: 0 }));
        g.linhas.push(l);
      }
      limites.forEach(([de, ate], i) => {
        const s = duracaoEm(r, de, ate, estado.agora);
        l.dias[i] += s;
        l.total += s;
        g.dias[i] += s;
        g.total += s;
        totais[i] += s;
      });
    }
    const lista = [...grupos.values()].sort((a, b) => b.total - a.total);
    lista.forEach((g) => g.linhas.sort((a, b) => b.total - a.total));
    return { grupos: lista, totais, total: totais.reduce((a, b) => a + b, 0) };
  });

  /** Cor do hachurado de cada dia da semana (null quando é um dia normal). */
  const hachuras = $derived(
    dias.map((d) => {
      const e = estado.especial(d);
      return e ? COR_ESPECIAL[e.tipo] : null;
    }),
  );

  const metaSemana = $derived(dias.reduce((s, d) => s + estado.metaDoDia(d), 0));

  $effect(() => {
    estado.garantirAnos(dias[0].getFullYear(), dias[6].getFullYear());
  });
  const hoje = $derived(new Date(estado.agora * 1000));
  const fmt = (s: number) => (s > 0 ? fmtDur(s) : "");
</script>

<div class="topo">
  <div class="navegacao">
    <button class="icone" onclick={() => (semana = somarDias(semana, -7))} title="Semana anterior">‹</button>
    <h1>{fmtDataCurta(dias[0])} a {fmtDataCurta(dias[6])}</h1>
    <button class="icone" onclick={() => (semana = somarDias(semana, 7))} title="Próxima semana">›</button>
    {#if !mesmoDia(semana, inicioSemana(hoje))}
      <button class="icone texto" onclick={() => (semana = inicioSemana(new Date()))}>Semana atual</button>
    {/if}
  </div>
  <div class="direita">
    {#if modo === "calendario"}
      <label class="secundario fds"><input type="checkbox" bind:checked={verFimDeSemana} /> Fim de semana</label>
    {/if}
    <div class="alternar">
      <button class:ativo={modo === "calendario"} onclick={() => (modo = "calendario")}>Calendário</button>
      <button class:ativo={modo === "tabela"} onclick={() => (modo = "tabela")}>Tabela</button>
    </div>
    <div class="secundario">
      <strong class="num total">{fmtDur(grade.total)}</strong>
      {#if metaSemana > 0} de {fmtDur(metaSemana)}{/if}
    </div>
  </div>
</div>

{#if modo === "calendario"}
  <CalendarioSemana {semana} {registros} {verFimDeSemana} />
{:else}
<div class="cartao tabela">
  <table>
    <thead>
      <tr>
        <th class="nome">Atividade</th>
        {#each dias as d, i}
          <th class:hoje={mesmoDia(d, hoje)} class:hachurado={!!hachuras[i]} style:--hachura={hachuras[i]}>
            <button class="dia" onclick={() => onabrirdia(d)} title={estado.especial(d)?.nome ?? "Ver o dia"}>
              {DIAS_CURTOS[i]} <span class="secundario">{d.getDate()}</span>
            </button>
            {#if estado.especial(d)}<div><SeloEspecial dia={estado.especial(d)} compacto /></div>{/if}
          </th>
        {/each}
        <th>Total</th>
      </tr>
    </thead>
    <tbody>
      {#each grade.grupos as g}
        <tr class="grupo">
          <td class="nome"><span class="ponto" style:background={g.cor}></span> {g.nome}</td>
          {#each g.dias as s, i}<td class="num" class:hachurado={!!hachuras[i]} style:--hachura={hachuras[i]}>{fmt(s)}</td>{/each}
          <td class="num">{fmt(g.total)}</td>
        </tr>
        {#each g.linhas as l (l.id)}
          <tr>
            <td class="nome atividade">
              <IconeTipo atividadeId={l.id} />
              <button class="link" onclick={() => (estado.atividadeAberta = l.id)}>{l.nome}</button>
            </td>
            {#each l.dias as s, i}<td class="num" class:hachurado={!!hachuras[i]} style:--hachura={hachuras[i]}>{fmt(s)}</td>{/each}
            <td class="num forte">{fmt(l.total)}</td>
          </tr>
        {/each}
      {:else}
        <tr><td colspan="9" class="vazio secundario">Nenhum registro nesta semana.</td></tr>
      {/each}
    </tbody>
    <tfoot>
      <tr>
        <td class="nome">Total</td>
        {#each grade.totais as s, i}<td class="num" class:hachurado={!!hachuras[i]} style:--hachura={hachuras[i]}>{fmt(s)}</td>{/each}
        <td class="num">{fmtDur(grade.total)}</td>
      </tr>
    </tfoot>
  </table>
</div>
{/if}

<style>
  .topo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .direita {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .fds {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }

  .alternar {
    display: flex;
  }

  .alternar button {
    border-radius: 0;
    padding: 5px 12px;
    font-size: 13px;
  }

  .alternar button:first-child {
    border-radius: 6px 0 0 6px;
  }

  .alternar button:last-child {
    border-radius: 0 6px 6px 0;
    border-left: none;
  }

  .alternar button.ativo {
    background: var(--destaque);
    border-color: var(--destaque);
    color: var(--destaque-texto);
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
  }

  .total {
    font-size: 20px;
    color: var(--texto);
  }

  .tabela {
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 8px 10px;
    text-align: right;
    border-bottom: 1px solid var(--borda);
    white-space: nowrap;
  }

  th {
    font-weight: 500;
    color: var(--texto-2);
  }

  th.hoje {
    color: var(--destaque);
  }

  .dia {
    background: none;
    border: none;
    padding: 2px 4px;
    color: inherit;
  }

  .nome {
    text-align: left;
    width: 40%;
  }

  .grupo td {
    font-weight: 600;
    background-color: var(--superficie-2);
  }

  .atividade {
    padding-left: 30px;
  }

  .forte {
    font-weight: 600;
  }

  tfoot td {
    font-weight: 600;
    border-bottom: none;
  }

  .vazio {
    text-align: center;
    padding: 30px;
  }
</style>
