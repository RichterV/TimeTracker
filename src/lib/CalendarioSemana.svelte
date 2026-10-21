<script lang="ts">
  import { api, type Registro } from "./api";
  import { estado } from "./estado.svelte";
  import IconeTipo from "./IconeTipo.svelte";
  import RegistroModal from "./RegistroModal.svelte";
  import SeloEspecial from "./SeloEspecial.svelte";
  import { COR_ESPECIAL } from "./especiais";
  import { DIAS_CURTOS, diaDaSemana, duracaoEm, fmtDur, fmtHora, mesmoDia, seg, somarDias } from "./tempo";

  let {
    semana,
    registros,
    verFimDeSemana,
  }: { semana: Date; registros: Registro[]; verFimDeSemana: boolean } = $props();

  const PX_HORA = 56;
  const PX_SEG = PX_HORA / 3600;
  /** Passo do arraste: 5 minutos. */
  const PASSO = 300;

  const ajustar = (t: number) => Math.round(t / PASSO) * PASSO;
  const horaDe = (hhmm: string) => {
    const [h, m] = hhmm.split(":").map(Number);
    return h + m / 60;
  };

  type Dia = { data: Date; ini: number; fim: number; nome: string; util: boolean; expediente: { inicio: string; fim: string }[] };

  const todos = $derived<Dia[]>(
    Array.from({ length: 7 }, (_, i) => {
      const data = somarDias(semana, i);
      return {
        data,
        ini: seg(data),
        fim: seg(somarDias(data, 1)),
        nome: DIAS_CURTOS[i],
        // feriado/folga/facultativo deixam o dia com fundo de dia não útil
        util: (estado.config?.dias_uteis.includes(diaDaSemana(data)) ?? i < 5) && estado.expedienteDoDia(data).length > 0,
        expediente: estado.expedienteDoDia(data),
      };
    }),
  );

  // dias úteis sempre; demais dias quando pedido ou quando têm registro
  const dias = $derived(
    todos.filter((d) => verFimDeSemana || d.util || estado.config?.dias_uteis.includes(diaDaSemana(d.data)) || registros.some((r) => duracaoEm(r, d.ini, d.fim, estado.agora) > 0)),
  );

  // ---------- arraste ----------

  type Modo = "mover" | "topo" | "base" | "criar";
  type Arraste = {
    modo: Modo;
    registro: Registro | null;
    bloqueado: boolean;
    x0: number;
    y0: number;
    dia0: number;
    ini0: number;
    fim0: number;
    inicio: number;
    fim: number;
    moveu: boolean;
  };

  let arraste = $state<Arraste | null>(null);
  /** Ajuste já salvo, mostrado até a lista de registros recarregar. */
  let pendente = $state<{ id: number; inicio: number; fim: number } | null>(null);
  let colunas = $state<HTMLElement[]>([]);
  let editando = $state<Registro | null>(null);
  let novo = $state<{ inicio: number; fim: number; dia: Date } | null>(null);
  let erro = $state("");
  let cliqueTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    registros;
    pendente = null;
  });

  const visiveis = $derived(
    registros.map((r) => {
      const o = arraste?.registro?.id === r.id && arraste.moveu && !arraste.bloqueado ? arraste : pendente?.id === r.id ? pendente : null;
      return o ? { ...r, inicio: o.inicio, fim: o.fim } : r;
    }),
  );

  // ---------- faixa de horas ----------

  const faixa = $derived.by(() => {
    const exp = estado.config?.expediente ?? [];
    let h0 = exp.length ? Math.floor(Math.min(...exp.map((i) => horaDe(i.inicio)))) - 1 : 8;
    let h1 = exp.length ? Math.ceil(Math.max(...exp.map((i) => horaDe(i.fim)))) + 1 : 18;
    for (const d of dias) {
      for (const r of visiveis) {
        const ini = Math.max(r.inicio, d.ini);
        const fim = Math.min(r.fim ?? estado.agora, d.fim);
        if (fim <= ini) continue;
        h0 = Math.min(h0, Math.floor((ini - d.ini) / 3600));
        h1 = Math.max(h1, Math.ceil((fim - d.ini) / 3600));
      }
    }
    if (novo) {
      const d = dias.find((x) => novo!.inicio >= x.ini && novo!.inicio < x.fim);
      if (d) h1 = Math.max(h1, Math.ceil((novo.fim - d.ini) / 3600));
    }
    h0 = Math.max(0, h0);
    h1 = Math.min(24, Math.max(h1, h0 + 1));
    return { h0, h1, horas: Array.from({ length: h1 - h0 }, (_, i) => h0 + i) };
  });

  const topo = (d: Dia, t: number) => (t - d.ini - faixa.h0 * 3600) * PX_SEG;

  // ---------- blocos com faixas lado a lado quando se sobrepõem ----------

  type Bloco = { r: Registro; ini: number; fim: number; faixa: number; faixas: number };

  function blocos(d: Dia): Bloco[] {
    const itens: Bloco[] = visiveis
      .map((r) => ({ r, ini: Math.max(r.inicio, d.ini), fim: Math.min(r.fim ?? estado.agora, d.fim), faixa: 0, faixas: 1 }))
      .filter((b) => b.fim > b.ini)
      .sort((a, b) => a.ini - b.ini || b.fim - a.fim);
    const saida: Bloco[] = [];
    let grupo: Bloco[] = [];
    let fimGrupo = 0;
    const fechar = () => {
      const fins: number[] = [];
      for (const b of grupo) {
        let f = fins.findIndex((x) => x <= b.ini);
        if (f < 0) f = fins.push(0) - 1;
        fins[f] = b.fim;
        b.faixa = f;
      }
      grupo.forEach((b) => (b.faixas = fins.length));
      saida.push(...grupo);
      grupo = [];
    };
    for (const b of itens) {
      if (grupo.length && b.ini >= fimGrupo) fechar();
      grupo.push(b);
      fimGrupo = Math.max(fimGrupo, b.fim);
    }
    fechar();
    return saida;
  }

  const tipoDe = (atividadeId: number) => {
    const tipoId = estado.buscarAtividade(atividadeId)?.atividade.tipo_id;
    return estado.tipos.find((t) => t.id === tipoId);
  };

  const totalDia = (d: Dia) => registros.reduce((s, r) => s + duracaoEm(r, d.ini, d.fim, estado.agora), 0);

  // ---------- posição do ponteiro ----------

  function segundosEm(clientY: number, i: number) {
    const rect = colunas[i].getBoundingClientRect();
    return dias[i].ini + faixa.h0 * 3600 + (clientY - rect.top) / PX_SEG;
  }

  function colunaEm(clientX: number) {
    let melhor = 0;
    colunas.forEach((c, i) => {
      if (c && clientX >= c.getBoundingClientRect().left) melhor = i;
    });
    return melhor;
  }

  function iniciarArrasteBloco(e: PointerEvent, b: Bloco, modo: Modo, i: number) {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    const fim = b.r.fim ?? estado.agora;
    arraste = {
      modo,
      registro: b.r,
      bloqueado: b.r.fim === null, // timer rodando: só clique
      x0: e.clientX,
      y0: e.clientY,
      dia0: i,
      ini0: b.r.inicio,
      fim0: fim,
      inicio: b.r.inicio,
      fim,
      moveu: false,
    };
  }

  function iniciarCriacao(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    e.preventDefault();
    const t = Math.floor(segundosEm(e.clientY, i) / PASSO) * PASSO;
    arraste = {
      modo: "criar",
      registro: null,
      bloqueado: false,
      x0: e.clientX,
      y0: e.clientY,
      dia0: i,
      ini0: t,
      fim0: t,
      inicio: t,
      fim: t + PASSO,
      moveu: false,
    };
  }

  function mover(e: PointerEvent) {
    const a = arraste;
    if (!a) return;
    if (!a.moveu && Math.hypot(e.clientX - a.x0, e.clientY - a.y0) < 4) return;
    a.moveu = true;
    if (a.bloqueado) return;
    const t = ajustar(segundosEm(e.clientY, a.dia0));
    if (a.modo === "mover") {
      const i = colunaEm(e.clientX);
      const delta = ajustar((e.clientY - a.y0) / PX_SEG) + (dias[i].ini - dias[a.dia0].ini);
      a.inicio = a.ini0 + delta;
      a.fim = a.fim0 + delta;
    } else if (a.modo === "topo") {
      a.inicio = Math.min(t, a.fim0 - PASSO);
    } else if (a.modo === "base") {
      a.fim = Math.max(t, a.ini0 + PASSO);
    } else {
      a.inicio = Math.min(a.ini0, t);
      a.fim = Math.max(a.ini0 + PASSO, t);
    }
  }

  async function soltar() {
    const a = arraste;
    if (!a) return;
    arraste = null;
    if (!a.moveu) {
      if (a.registro) clique(a.registro);
      return;
    }
    if (a.bloqueado) return;
    if (a.modo === "criar") {
      novo = { inicio: a.inicio, fim: a.fim, dia: dias[a.dia0].data };
      return;
    }
    const r = a.registro!;
    if (a.inicio === r.inicio && a.fim === r.fim) return;
    pendente = { id: r.id, inicio: a.inicio, fim: a.fim };
    erro = "";
    try {
      await api.salvarRegistro({ id: r.id, atividade_id: r.atividade_id, inicio: a.inicio, fim: a.fim, nota: r.nota });
    } catch (x) {
      pendente = null;
      erro = String(x);
    }
  }

  /** Um clique edita o registro; dois cliques abrem o painel da atividade. */
  function clique(r: Registro) {
    if (cliqueTimer) {
      clearTimeout(cliqueTimer);
      cliqueTimer = null;
      estado.atividadeAberta = r.atividade_id;
      return;
    }
    cliqueTimer = setTimeout(() => {
      cliqueTimer = null;
      editando = r;
    }, 280);
  }

  const hoje = $derived(new Date(estado.agora * 1000));
</script>

<svelte:window onpointermove={mover} onpointerup={soltar} />

{#if erro}<p class="erro">{erro}</p>{/if}

<div class="calendario cartao" class:arrastando={arraste?.moveu}>
  <div class="cabecalho" style:grid-template-columns="48px repeat({dias.length}, 1fr)">
    <div></div>
    {#each dias as d}
      {@const total = totalDia(d)}
      <div class="dia-cab" class:hoje={mesmoDia(d.data, hoje)}>
        <span class="numero">{d.data.getDate()}</span>
        <span class="nome-dia">{d.nome}</span>
        {#if total > 0}<span class="total num">{fmtDur(total)}</span>{/if}
        {#if estado.especial(d.data)}<div class="especial"><SeloEspecial dia={estado.especial(d.data)} compacto /></div>{/if}
      </div>
    {/each}
  </div>

  <div class="corpo" style:grid-template-columns="48px repeat({dias.length}, 1fr)">
    <div class="horas" style:height="{(faixa.h1 - faixa.h0) * PX_HORA}px">
      {#each faixa.horas as h, i}
        <span style:top="{i * PX_HORA}px">{h}</span>
      {/each}
    </div>

    {#each dias as d, i (d.ini)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="coluna"
        class:fora={!d.util}
        bind:this={colunas[i]}
        style:height="{(faixa.h1 - faixa.h0) * PX_HORA}px"
        style:background-size="100% {PX_HORA}px"
        onpointerdown={(e) => iniciarCriacao(e, i)}
      >
        {#if estado.especial(d.data)}
          {@const e = estado.especial(d.data)!}
          <!-- meio período: hachura só até o início do expediente -->
          <div
            class="hachura hachurado"
            style:--hachura={COR_ESPECIAL[e.tipo]}
            style:height={e.a_partir ? `${Math.max(0, (horaDe(e.a_partir) - faixa.h0) * PX_HORA)}px` : "100%"}
          ></div>
        {/if}
        {#if d.util}
          {#each d.expediente as exp}
            <div
              class="expediente"
              style:top="{(horaDe(exp.inicio) - faixa.h0) * PX_HORA}px"
              style:height="{(horaDe(exp.fim) - horaDe(exp.inicio)) * PX_HORA}px"
            ></div>
          {/each}
        {/if}

        {#each blocos(d) as b (b.r.id)}
          {@const tipo = tipoDe(b.r.atividade_id)}
          {@const cor = tipo?.cor ?? b.r.cor}
          {@const altura = Math.max((b.fim - b.ini) * PX_SEG, 16)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="bloco"
            class:rodando={b.r.fim === null}
            class:ativo={arraste?.registro?.id === b.r.id && arraste.moveu}
            style:--cor={cor}
            style:top="{topo(d, b.ini)}px"
            style:height="{altura}px"
            style:left="calc({(b.faixa / b.faixas) * 100}% + 2px)"
            style:width="calc({100 / b.faixas}% - 4px)"
            title="{b.r.atividade} · {b.r.projeto}&#10;{fmtHora(b.r.inicio)}–{b.r.fim ? fmtHora(b.r.fim) : 'agora'}{b.r.nota ? `\n${b.r.nota}` : ''}"
            onpointerdown={(e) => iniciarArrasteBloco(e, b, "mover", i)}
          >
            {#if b.r.fim !== null}
              <div class="alca topo-alca" onpointerdown={(e) => iniciarArrasteBloco(e, b, "topo", i)}></div>
            {/if}
            <div class="linha forte">
              <IconeTipo atividadeId={b.r.atividade_id} tamanho={13} />
              <span>{b.r.atividade}</span>
            </div>
            <div class="linha">{b.r.projeto}</div>
            <div class="linha num">
              {fmtHora(b.r.inicio)}–{b.r.fim ? fmtHora(b.r.fim) : "agora"} · {fmtDur((b.r.fim ?? estado.agora) - b.r.inicio)}
            </div>
            {#if b.r.nota}<div class="linha nota">{b.r.nota}</div>{/if}
            {#if b.r.fim !== null}
              <div class="alca base-alca" onpointerdown={(e) => iniciarArrasteBloco(e, b, "base", i)}></div>
            {/if}
          </div>
        {/each}

        {#if arraste?.modo === "criar" && arraste.moveu && arraste.dia0 === i}
          <div class="bloco previa" style:top="{topo(d, arraste.inicio)}px" style:height="{(arraste.fim - arraste.inicio) * PX_SEG}px">
            <div class="linha num">{fmtHora(arraste.inicio)}–{fmtHora(arraste.fim)} · {fmtDur(arraste.fim - arraste.inicio)}</div>
          </div>
        {/if}

        {#if mesmoDia(d.data, hoje)}
          {@const y = topo(d, estado.agora)}
          {#if y >= 0 && y <= (faixa.h1 - faixa.h0) * PX_HORA}
            <div class="agora" style:top="{y}px"></div>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
</div>

<p class="dica secundario">
  Arraste no vazio para lançar tempo · arraste o bloco para mover, as bordas para ajustar · clique edita, duplo clique abre a atividade
</p>

{#if editando}
  <RegistroModal registro={editando} dia={new Date(editando.inicio * 1000)} onfechar={() => (editando = null)} />
{/if}

{#if novo}
  <RegistroModal registro={null} dia={novo.dia} intervalo={novo} onfechar={() => (novo = null)} />
{/if}

<style>
  .calendario {
    overflow: auto;
    max-height: calc(100vh - 250px);
    position: relative;
  }

  .calendario.arrastando {
    cursor: grabbing;
  }

  .cabecalho,
  .corpo {
    display: grid;
    min-width: 560px;
  }

  .cabecalho {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--superficie);
    border-bottom: 1px solid var(--borda);
  }

  .dia-cab {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 8px 10px;
    border-left: 1px solid var(--borda);
  }

  .numero {
    font-size: 20px;
    font-weight: 300;
  }

  .nome-dia {
    color: var(--texto-2);
    font-size: 12px;
  }

  .dia-cab {
    flex-wrap: wrap;
  }

  .especial {
    flex-basis: 100%;
    min-width: 0;
    margin-top: 2px;
  }

  .hachura {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    pointer-events: none;
  }

  .dia-cab .total {
    margin-left: auto;
    font-size: 12px;
    font-weight: 600;
  }

  .dia-cab.hoje {
    box-shadow: inset 0 3px 0 var(--destaque);
  }

  .dia-cab.hoje .numero,
  .dia-cab.hoje .nome-dia {
    color: var(--destaque);
    font-weight: 600;
  }

  .horas {
    position: relative;
  }

  .horas span {
    position: absolute;
    right: 8px;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--texto-2);
  }

  .horas span:first-child {
    transform: none;
  }

  .coluna {
    position: relative;
    border-left: 1px solid var(--borda);
    background-color: color-mix(in srgb, var(--fundo) 70%, var(--superficie));
    background-image: linear-gradient(to bottom, var(--borda) 1px, transparent 1px);
    cursor: cell;
    touch-action: none;
  }

  .coluna.fora {
    background-color: var(--fundo);
  }

  .expediente {
    position: absolute;
    left: 0;
    right: 0;
    background: var(--superficie);
    opacity: 0.6;
    pointer-events: none;
  }

  .bloco {
    position: absolute;
    z-index: 1;
    overflow: hidden;
    padding: 3px 6px;
    border-radius: 4px;
    border-left: 3px solid var(--cor);
    background: color-mix(in srgb, var(--cor) 22%, var(--superficie));
    font-size: 11.5px;
    line-height: 1.35;
    cursor: grab;
    user-select: none;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.12);
  }

  .bloco:hover {
    background: color-mix(in srgb, var(--cor) 32%, var(--superficie));
  }

  .bloco.ativo {
    opacity: 0.85;
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.25);
    z-index: 2;
  }

  .bloco.rodando {
    cursor: pointer;
    border-left-style: dashed;
  }

  .bloco.previa {
    left: 2px;
    right: 2px;
    --cor: var(--destaque);
    background: color-mix(in srgb, var(--destaque) 25%, transparent);
    border: 1px dashed var(--destaque);
    pointer-events: none;
  }

  .linha {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .forte {
    font-weight: 600;
  }

  .forte span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nota {
    color: var(--texto-2);
    font-style: italic;
  }

  .alca {
    position: absolute;
    left: 0;
    right: 0;
    height: 6px;
    cursor: ns-resize;
  }

  .topo-alca {
    top: 0;
  }

  .base-alca {
    bottom: 0;
  }

  .agora {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--perigo);
    z-index: 2;
    pointer-events: none;
  }

  .agora::before {
    content: "";
    position: absolute;
    left: -4px;
    top: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--perigo);
  }

  .dica {
    font-size: 12px;
    margin: 8px 0 0;
  }

  .erro {
    margin: 0 0 8px;
  }
</style>
