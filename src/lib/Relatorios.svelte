<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { api } from "./api";
  import { estado } from "./estado.svelte";
  import { fmtDiaLongo, gerarPdf, montarRelatorio, type Relatorio } from "./relatorio";
  import { deInputs, fmtDataCurta, fmtDur, inicioSemana, paraInputData, somarDias } from "./tempo";

  const hoje = () => new Date();
  const ultimoDiaDoMes = (d: Date) => new Date(d.getFullYear(), d.getMonth() + 1, 0);

  const ATALHOS: { nome: string; periodo: () => [Date, Date] }[] = [
    { nome: "Esta semana", periodo: () => [inicioSemana(hoje()), somarDias(inicioSemana(hoje()), 6)] },
    {
      nome: "Semana passada",
      periodo: () => [somarDias(inicioSemana(hoje()), -7), somarDias(inicioSemana(hoje()), -1)],
    },
    {
      nome: "Este mês",
      periodo: () => [new Date(hoje().getFullYear(), hoje().getMonth(), 1), ultimoDiaDoMes(hoje())],
    },
    {
      nome: "Mês passado",
      periodo: () => {
        const ini = new Date(hoje().getFullYear(), hoje().getMonth() - 1, 1);
        return [ini, ultimoDiaDoMes(ini)];
      },
    },
  ];

  let de = $state(paraInputData(inicioSemana(hoje())));
  let ate = $state(paraInputData(somarDias(inicioSemana(hoje()), 6)));
  let relatorio = $state<Relatorio | null>(null);
  let carregando = $state(false);
  let abrirDepois = $state(true);
  let mensagem = $state("");
  let erro = $state("");

  const dataDe = (iso: string) => new Date(deInputs(iso, "00:00") * 1000);

  function aplicar(periodo: [Date, Date]) {
    de = paraInputData(periodo[0]);
    ate = paraInputData(periodo[1]);
  }

  $effect(() => {
    estado.versao;
    if (!de || !ate || ate < de) return;
    const [a, b] = [dataDe(de), dataDe(ate)];
    carregando = true;
    montarRelatorio(a, b)
      .then((r) => (relatorio = r))
      .catch((e) => (erro = String(e)))
      .finally(() => (carregando = false));
  });

  async function exportar() {
    if (!relatorio) return;
    erro = mensagem = "";
    const caminho = await save({
      title: "Salvar relatório",
      defaultPath: `relatorio-8pace_${de}_a_${ate}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!caminho) return;
    const final = caminho.toLowerCase().endsWith(".pdf") ? caminho : `${caminho}.pdf`;
    try {
      await api.gravarPdf(final, gerarPdf(relatorio), abrirDepois);
      mensagem = `Relatório salvo em ${final}`;
    } catch (e) {
      erro = String(e);
    }
  }

  const fmtQuando = (ts: number) =>
    new Date(ts * 1000).toLocaleString("pt-BR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
</script>

<div class="topo">
  <div class="periodo">
    {#each ATALHOS as a}
      <button onclick={() => aplicar(a.periodo())}>{a.nome}</button>
    {/each}
    <span class="separador"></span>
    <input type="date" bind:value={de} max={ate} />
    <span class="secundario">até</span>
    <input type="date" bind:value={ate} min={de} />
  </div>
  <div class="exportar">
    <label class="secundario"><input type="checkbox" bind:checked={abrirDepois} /> Abrir após salvar</label>
    <button class="primario" onclick={exportar} disabled={!relatorio || relatorio.dias.length === 0}>Exportar PDF</button>
  </div>
</div>

{#if erro}<p class="erro">{erro}</p>{/if}
{#if mensagem}<p class="secundario mensagem">{mensagem}</p>{/if}

<section class="cartao folha" class:carregando>
  {#if relatorio}
    <header>
      <h2>Relatório de atividades</h2>
      <span class="secundario">
        {fmtDataCurta(relatorio.de)} a {fmtDataCurta(relatorio.ate)} · total <strong class="num">{fmtDur(relatorio.total)}</strong>
      </span>
    </header>

    {#if relatorio.dias.length === 0}
      <p class="vazio secundario">Nenhuma atividade registrada no período.</p>
    {:else}
      <div class="indicadores">
        <div><span>Dias trabalhados</span><strong class="num">{relatorio.dias.length}</strong></div>
        <div><span>Projetos</span><strong class="num">{relatorio.projetos.length}</strong></div>
        <div><span>Atividades</span><strong class="num">{relatorio.atividades}</strong></div>
        <div>
          <span>Média por dia</span>
          <strong class="num">{fmtDur(Math.round(relatorio.total / relatorio.dias.length))}</strong>
        </div>
      </div>

      <h3>Horas por projeto</h3>
      <div class="projetos">
        {#each relatorio.projetos as p}
          <span class="ponto" style:background={p.cor}></span>
          <span>{p.nome}</span>
          <span class="barra"><span style:width="{(p.seg / relatorio.projetos[0].seg) * 100}%" style:background={p.cor}></span></span>
          <strong class="num dir">{fmtDur(p.seg)}</strong>
          <span class="num dir secundario">{Math.round((p.seg / relatorio.total) * 100)}%</span>
        {/each}
      </div>

      <h3>Atividades por dia</h3>
      <table>
        <thead>
          <tr><th>Projeto</th><th>Atividade</th><th>Tipo</th><th>Status</th><th class="dir">Horas</th></tr>
        </thead>
        <tbody>
          {#each relatorio.dias as dia}
            <tr class="dia">
              <td colspan="4">{fmtDiaLongo(dia.data)}</td>
              <td class="num dir">{fmtDur(dia.total)}</td>
            </tr>
            {#each dia.linhas as l}
              <tr>
                <td class="secundario">{l.projeto}</td>
                <td>{l.atividade}</td>
                <td><span class="ponto" style:background={l.tipoCor}></span> {l.tipo}</td>
                <td style:color={l.statusCor}>{l.status}</td>
                <td class="num dir forte">{fmtDur(l.seg)}</td>
              </tr>
            {/each}
          {/each}
        </tbody>
        <tfoot>
          <tr><td colspan="4">Total do período</td><td class="num dir">{fmtDur(relatorio.total)}</td></tr>
        </tfoot>
      </table>

      {#if relatorio.comentarios.length > 0}
        <h3>Comentários do período</h3>
        {#each relatorio.comentarios as g}
          <div class="grupo">
            <strong>{g.atividade}</strong> <span class="secundario">· {g.projeto}</span>
            {#each g.itens as c}
              <div class="comentario">
                <span class="secundario num">{fmtQuando(c.criado)}</span>
                <p>{c.texto}</p>
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    {/if}
  {/if}
</section>

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 14px;
  }

  .periodo,
  .exportar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .periodo button {
    font-size: 13px;
    padding: 5px 10px;
  }

  .separador {
    width: 1px;
    height: 22px;
    background: var(--borda);
    margin: 0 6px;
  }

  .exportar label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }

  .mensagem,
  .erro {
    margin: 0 0 10px;
    font-size: 13px;
    user-select: text;
  }

  .folha {
    padding: 18px 20px;
    max-width: 980px;
    margin: 0 auto;
    transition: opacity 0.15s;
  }

  .folha.carregando {
    opacity: 0.6;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  h2 {
    margin: 0;
    font-size: 17px;
  }

  h3 {
    margin: 22px 0 8px;
    font-size: 14px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th,
  td {
    text-align: left;
    padding: 6px 8px;
    border-bottom: 1px solid var(--borda);
    vertical-align: top;
  }

  th {
    color: var(--texto-2);
    font-weight: 500;
  }

  .dir {
    text-align: right;
  }

  tr.dia td {
    background: color-mix(in srgb, var(--destaque) 12%, transparent);
    color: var(--destaque);
    font-weight: 600;
    border-bottom: none;
    padding-top: 8px;
    padding-bottom: 8px;
  }

  .forte {
    font-weight: 600;
  }

  .indicadores {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }

  .indicadores div {
    background: var(--superficie-2);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .indicadores span {
    font-size: 11px;
    color: var(--texto-2);
    text-transform: uppercase;
  }

  .indicadores strong {
    font-size: 18px;
  }

  .projetos {
    display: grid;
    grid-template-columns: auto minmax(120px, 1fr) 2fr auto auto;
    align-items: center;
    gap: 6px 10px;
    font-size: 13px;
  }

  .barra {
    height: 6px;
    background: var(--superficie-2);
    border-radius: 3px;
    overflow: hidden;
  }

  .barra span {
    display: block;
    height: 100%;
    border-radius: 3px;
  }

  tfoot td {
    font-weight: 600;
    border-bottom: none;
  }

  .grupo {
    margin-bottom: 12px;
    font-size: 13px;
  }

  .comentario {
    margin: 4px 0 0 12px;
  }

  .comentario p {
    margin: 2px 0 0;
    white-space: pre-wrap;
    user-select: text;
  }

  .vazio {
    text-align: center;
    padding: 30px 0;
  }
</style>
