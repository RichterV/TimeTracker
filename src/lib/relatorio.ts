import { jsPDF } from "jspdf";
import autoTable, { type CellHookData, type RowInput } from "jspdf-autotable";
import { api } from "./api";
import { estado } from "./estado.svelte";
import { infoStatus } from "./status";
import { duracaoEm, fmtDur, inicioDia, seg, somarDias } from "./tempo";

export type LinhaRelatorio = {
  data: Date;
  atividadeId: number;
  projeto: string;
  projetoCor: string;
  atividade: string;
  tipo: string;
  tipoCor: string;
  status: string;
  statusCor: string;
  seg: number;
};

export type DiaRelatorio = { data: Date; total: number; linhas: LinhaRelatorio[] };

export type ProjetoRelatorio = { nome: string; cor: string; seg: number };

export type GrupoComentarios = {
  atividade: string;
  projeto: string;
  itens: { criado: number; texto: string }[];
};

export type Relatorio = {
  de: Date;
  ate: Date;
  dias: DiaRelatorio[];
  projetos: ProjetoRelatorio[];
  total: number;
  atividades: number;
  comentarios: GrupoComentarios[];
};

/** Texto do comentário sem os tokens de anexo (viram "[anexo: nome]"). */
export const textoSimples = (texto: string) =>
  texto.replace(/\{\{anexo:(\d+)(?:\|([^}]*))?\}\}/g, (_, id, nome) => `[anexo: ${nome || id}]`).trim();

/** "Segunda-feira, 20/10/2026" */
export const fmtDiaLongo = (d: Date) => {
  const s = d.toLocaleDateString("pt-BR", { weekday: "long", day: "2-digit", month: "2-digit", year: "numeric" });
  return s.charAt(0).toUpperCase() + s.slice(1);
};

/** Atividades por dia no período [de, ate] (datas inclusivas). */
export async function montarRelatorio(de: Date, ate: Date): Promise<Relatorio> {
  const ini = seg(inicioDia(de));
  const fim = seg(somarDias(inicioDia(ate), 1));
  const registros = await api.listarRegistros(ini, fim);

  const dias: DiaRelatorio[] = [];
  const projetos = new Map<string, ProjetoRelatorio>();
  for (let d = inicioDia(de); seg(d) < fim; d = somarDias(d, 1)) {
    const a = seg(d);
    const b = seg(somarDias(d, 1));
    const porAtividade = new Map<number, LinhaRelatorio>();
    for (const r of registros) {
      const s = duracaoEm(r, a, b, estado.agora);
      if (s <= 0) continue;
      let l = porAtividade.get(r.atividade_id);
      if (!l) {
        const info = estado.buscarAtividade(r.atividade_id)?.atividade;
        const tipo = estado.tipos.find((t) => t.id === info?.tipo_id);
        const status = info ? infoStatus(info.status) : null;
        l = {
          data: d,
          atividadeId: r.atividade_id,
          projeto: r.projeto,
          projetoCor: r.cor,
          atividade: r.atividade,
          tipo: tipo?.nome ?? "",
          tipoCor: tipo?.cor ?? "#8a93a3",
          status: status?.nome ?? "",
          statusCor: status?.cor ?? "#8a93a3",
          seg: 0,
        };
        porAtividade.set(r.atividade_id, l);
      }
      l.seg += s;
      const p = projetos.get(r.projeto) ?? { nome: r.projeto, cor: r.cor, seg: 0 };
      p.seg += s;
      projetos.set(r.projeto, p);
    }
    if (porAtividade.size === 0) continue;
    const linhas = [...porAtividade.values()].sort(
      (x, y) => x.projeto.localeCompare(y.projeto) || x.atividade.localeCompare(y.atividade),
    );
    dias.push({ data: d, total: linhas.reduce((s, l) => s + l.seg, 0), linhas });
  }

  const ids = [...new Set(dias.flatMap((d) => d.linhas.map((l) => l.atividadeId)))];
  const comentarios: GrupoComentarios[] = [];
  for (const id of ids) {
    const itens = (await api.listarComentarios(id))
      .filter((c) => c.criado >= ini && c.criado < fim)
      .map((c) => ({ criado: c.criado, texto: textoSimples(c.texto) }));
    if (itens.length === 0) continue;
    const l = dias.flatMap((d) => d.linhas).find((x) => x.atividadeId === id)!;
    comentarios.push({ atividade: l.atividade, projeto: l.projeto, itens });
  }

  return {
    de,
    ate,
    dias,
    projetos: [...projetos.values()].sort((a, b) => b.seg - a.seg),
    total: dias.reduce((s, d) => s + d.total, 0),
    atividades: ids.length,
    comentarios,
  };
}

// ---------- PDF ----------

type RGB = [number, number, number];

const rgb = (hex: string): RGB => {
  const n = parseInt(hex.replace("#", "").slice(0, 6), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};

/** Paleta escura (mesmos tons do tema escuro do app). */
const COR = {
  fundo: rgb("#16181c"),
  destaque: rgb("#5b8cf5"),
  texto: rgb("#e6e8ec"),
  secundario: rgb("#9aa2ae"),
  borda: rgb("#343943"),
  faixaDia: rgb("#22304f"),
  textoDia: rgb("#a9c1ff"),
  fundoCartao: rgb("#1f2228"),
};

/** Clareia a cor para ter contraste sobre o fundo escuro. */
const clarear = ([r, g, b]: RGB, f = 0.3): RGB => [r + (255 - r) * f, g + (255 - g) * f, b + (255 - b) * f].map(Math.round) as RGB;

const fmtDataCompleta = (d: Date) => d.toLocaleDateString("pt-BR");
const fmtDataHora = (ts: number) =>
  new Date(ts * 1000).toLocaleString("pt-BR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });

/** Gera o PDF do relatório (A4 retrato). */
export function gerarPdf(rel: Relatorio): Uint8Array {
  const doc = new jsPDF({ unit: "mm", format: "a4" });
  const pintarFundo = () => {
    doc.setFillColor(...COR.fundo);
    doc.rect(0, 0, doc.internal.pageSize.getWidth(), doc.internal.pageSize.getHeight(), "F");
  };
  // toda página nova (inclusive as criadas pela tabela) já nasce com o fundo escuro
  const addPageOriginal = doc.addPage.bind(doc);
  doc.addPage = ((...args: Parameters<typeof doc.addPage>) => {
    const r = addPageOriginal(...args);
    pintarFundo();
    return r;
  }) as typeof doc.addPage;
  pintarFundo();
  const M = 14;
  const larguraPagina = doc.internal.pageSize.getWidth();
  const alturaPagina = doc.internal.pageSize.getHeight();
  const largura = larguraPagina - M * 2;
  const ultimoY = () => (doc as unknown as { lastAutoTable: { finalY: number } }).lastAutoTable.finalY;

  // ---- cabeçalho ----
  doc.setFillColor(...COR.destaque);
  doc.rect(0, 0, larguraPagina, 3, "F");

  doc.setFont("helvetica", "bold");
  doc.setFontSize(8);
  doc.setTextColor(...COR.destaque);
  doc.text("8PACE", M, 14);
  doc.setFontSize(18);
  doc.setTextColor(...COR.texto);
  doc.text("Relatório de atividades", M, 22);
  doc.setFont("helvetica", "normal");
  doc.setFontSize(10);
  doc.setTextColor(...COR.secundario);
  doc.text(`${fmtDataCompleta(rel.de)} a ${fmtDataCompleta(rel.ate)}`, M, 28);

  doc.setFontSize(8);
  doc.text("TOTAL NO PERÍODO", larguraPagina - M, 14, { align: "right" });
  doc.setFont("helvetica", "bold");
  doc.setFontSize(20);
  doc.setTextColor(...COR.texto);
  doc.text(fmtDur(rel.total), larguraPagina - M, 23, { align: "right" });

  // ---- indicadores ----
  const diasTrab = rel.dias.length;
  const indicadores: [string, string][] = [
    ["Dias trabalhados", String(diasTrab)],
    ["Projetos", String(rel.projetos.length)],
    ["Atividades", String(rel.atividades)],
    ["Média por dia", diasTrab ? fmtDur(Math.round(rel.total / diasTrab)) : "-"],
  ];
  const gap = 4;
  const larguraCartao = (largura - gap * (indicadores.length - 1)) / indicadores.length;
  let y = 34;
  indicadores.forEach(([rotulo, valor], i) => {
    const x = M + i * (larguraCartao + gap);
    doc.setFillColor(...COR.fundoCartao);
    doc.roundedRect(x, y, larguraCartao, 16, 2, 2, "F");
    doc.setFont("helvetica", "normal");
    doc.setFontSize(7.5);
    doc.setTextColor(...COR.secundario);
    doc.text(rotulo.toUpperCase(), x + 4, y + 6);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(13);
    doc.setTextColor(...COR.texto);
    doc.text(valor, x + 4, y + 12.5);
  });
  y += 24;

  const titulo = (texto: string, yy: number) => {
    doc.setFont("helvetica", "bold");
    doc.setFontSize(11);
    doc.setTextColor(...COR.texto);
    doc.text(texto, M, yy);
  };

  // ---- por projeto ----
  if (rel.projetos.length > 0) {
    titulo("Horas por projeto", y);
    const maior = rel.projetos[0].seg;
    autoTable(doc, {
      startY: y + 3,
      margin: { left: M, right: M },
      theme: "plain",
      head: [["Projeto", "", "Horas", "%"]],
      body: rel.projetos.map((p) => [p.nome, "", fmtDur(p.seg), `${Math.round((p.seg / rel.total) * 100)}%`]),
      styles: { fontSize: 8.5, cellPadding: { top: 1.6, bottom: 1.6, left: 2, right: 2 }, textColor: COR.texto },
      headStyles: { textColor: COR.secundario, fontStyle: "normal", fontSize: 7.5 },
      columnStyles: {
        0: { cellWidth: 70, cellPadding: { top: 1.6, bottom: 1.6, left: 6, right: 2 } },
        2: { cellWidth: 18, halign: "right", fontStyle: "bold" },
        3: { cellWidth: 14, halign: "right", textColor: COR.secundario },
      },
      didDrawCell: (d: CellHookData) => {
        if (d.section !== "body") return;
        const p = rel.projetos[d.row.index];
        if (d.column.index === 0) {
          doc.setFillColor(...rgb(p.cor));
          doc.circle(d.cell.x + 3, d.cell.y + d.cell.height / 2, 1.1, "F");
        }
        if (d.column.index === 1) {
          const w = (d.cell.width - 4) * (p.seg / maior);
          doc.setFillColor(...COR.borda);
          doc.roundedRect(d.cell.x + 2, d.cell.y + d.cell.height / 2 - 1.2, d.cell.width - 4, 2.4, 1.2, 1.2, "F");
          doc.setFillColor(...rgb(p.cor));
          doc.roundedRect(d.cell.x + 2, d.cell.y + d.cell.height / 2 - 1.2, Math.max(w, 2.4), 2.4, 1.2, 1.2, "F");
        }
      },
    });
    y = ultimoY() + 10;
  }

  // ---- atividades por dia ----
  if (y > alturaPagina - 40) {
    doc.addPage();
    y = 18;
  }
  titulo("Atividades por dia", y);

  type Meta = { tipo: "dia" } | { tipo: "linha"; l: LinhaRelatorio };
  const meta: Meta[] = [];
  const corpo: RowInput[] = [];
  for (const dia of rel.dias) {
    meta.push({ tipo: "dia" });
    corpo.push([
      {
        content: fmtDiaLongo(dia.data),
        colSpan: 4,
        styles: { fillColor: COR.faixaDia, textColor: COR.textoDia, fontStyle: "bold", fontSize: 9 },
      },
      {
        content: fmtDur(dia.total),
        styles: { fillColor: COR.faixaDia, textColor: COR.textoDia, fontStyle: "bold", fontSize: 9, halign: "right" },
      },
    ]);
    for (const l of dia.linhas) {
      meta.push({ tipo: "linha", l });
      corpo.push([l.projeto, l.atividade, l.tipo, l.status, fmtDur(l.seg)]);
    }
  }

  autoTable(doc, {
    startY: y + 3,
    margin: { left: M, right: M, top: 16 },
    theme: "plain",
    head: [["Projeto", "Atividade", "Tipo", "Status", "Horas"]],
    body: corpo,
    foot: [[{ content: "Total do período", colSpan: 4 }, fmtDur(rel.total)]],
    showFoot: "lastPage",
    styles: {
      fontSize: 8.5,
      cellPadding: { top: 2, bottom: 2, left: 2.5, right: 2.5 },
      textColor: COR.texto,
      valign: "middle",
      lineColor: COR.borda,
    },
    headStyles: { textColor: COR.secundario, fontStyle: "normal", fontSize: 7.5, lineWidth: { bottom: 0.3 } },
    footStyles: { fontStyle: "bold", fontSize: 9.5, lineWidth: { top: 0.4 }, lineColor: COR.secundario },
    columnStyles: {
      0: { cellWidth: 44 },
      2: { cellWidth: 30, cellPadding: { top: 2, bottom: 2, left: 6, right: 2 } },
      3: { cellWidth: 26 },
      4: { cellWidth: 16, halign: "right", fontStyle: "bold" },
    },
    didParseCell: (d: CellHookData) => {
      if (d.section === "foot" && d.column.index === 4) d.cell.styles.halign = "right";
      if (d.section !== "body") return;
      const m = meta[d.row.index];
      if (m.tipo !== "linha") return;
      d.cell.styles.lineWidth = { bottom: 0.15 };
      if (d.column.index === 3) d.cell.styles.textColor = clarear(rgb(m.l.statusCor));
      if (d.column.index === 0) d.cell.styles.textColor = COR.secundario;
    },
    didDrawCell: (d: CellHookData) => {
      if (d.section !== "body" || d.column.index !== 2) return;
      const m = meta[d.row.index];
      if (m.tipo !== "linha" || !m.l.tipo) return;
      doc.setFillColor(...rgb(m.l.tipoCor));
      doc.circle(d.cell.x + 3.2, d.cell.y + d.cell.height / 2, 1.1, "F");
    },
  });

  // ---- comentários ----
  if (rel.comentarios.length > 0) {
    y = ultimoY() + 12;
    const garantir = (altura: number) => {
      if (y + altura > alturaPagina - 16) {
        doc.addPage();
        y = 18;
      }
    };
    garantir(14);
    titulo("Comentários do período", y);
    y += 7;

    for (const g of rel.comentarios) {
      garantir(12);
      doc.setFillColor(...COR.destaque);
      doc.rect(M, y - 3.4, 0.8, 4.6, "F");
      doc.setFont("helvetica", "bold");
      doc.setFontSize(9.5);
      doc.setTextColor(...COR.texto);
      const cabecalho: string[] = doc.splitTextToSize(g.atividade, largura - 4);
      doc.text(cabecalho, M + 3, y);
      y += cabecalho.length * 4.4;
      doc.setFont("helvetica", "normal");
      doc.setFontSize(8);
      doc.setTextColor(...COR.secundario);
      doc.text(g.projeto, M + 3, y);
      y += 5;

      doc.setFontSize(8.8);
      for (const c of g.itens) {
        const linhas: string[] = doc.splitTextToSize(c.texto, largura - 8);
        garantir(9);
        doc.setFont("helvetica", "bold");
        doc.setFontSize(7.5);
        doc.setTextColor(...COR.secundario);
        doc.text(fmtDataHora(c.criado), M + 3, y);
        y += 4;
        doc.setFont("helvetica", "normal");
        doc.setFontSize(8.8);
        doc.setTextColor(...COR.texto);
        for (const linha of linhas) {
          garantir(4.2);
          doc.text(linha, M + 3, y);
          y += 4.2;
        }
        y += 2.5;
      }
      y += 4;
    }
  }

  // ---- rodapé ----
  const paginas = doc.getNumberOfPages();
  for (let p = 1; p <= paginas; p++) {
    doc.setPage(p);
    doc.setDrawColor(...COR.borda);
    doc.setLineWidth(0.2);
    doc.line(M, alturaPagina - 11, larguraPagina - M, alturaPagina - 11);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(7.5);
    doc.setTextColor(...COR.secundario);
    doc.text(`8Pace · ${fmtDataCompleta(rel.de)} a ${fmtDataCompleta(rel.ate)}`, M, alturaPagina - 7);
    doc.text(`Página ${p} de ${paginas}`, larguraPagina - M, alturaPagina - 7, { align: "right" });
  }

  return new Uint8Array(doc.output("arraybuffer"));
}
