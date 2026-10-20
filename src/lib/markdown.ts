import { convertFileSrc } from "@tauri-apps/api/core";
import { Marked } from "marked";
import type { Anexo } from "./api";

/** Token de anexo no texto: {{anexo:ID}} ou {{anexo:ID|nome}}. */
export const TOKEN_ANEXO = /\{\{anexo:(\d+)(?:\|[^}]*)?\}\}/g;

export const tokenAnexo = (a: Anexo) => `{{anexo:${a.id}|${a.nome.replace(/[{}|]/g, "")}}}`;

const escapar = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const marked = new Marked({
  gfm: true,
  breaks: true,
  renderer: {
    // HTML digitado aparece como texto, nunca é interpretado
    html: ({ text }) => escapar(text),
    // imagens externas não são carregadas; anexos usam o token próprio
    image: ({ text }) => escapar(text),
    link({ href, tokens }) {
      const texto = this.parser.parseInline(tokens);
      return /^(https?:|mailto:)/i.test(href)
        ? `<a href="${escapar(href)}" data-link title="${escapar(href)}">${texto}</a>`
        : texto;
    },
  },
});

function htmlAnexo(a: Anexo | undefined): string {
  if (!a) return `<span class="anexo-faltando">anexo removido</span>`;
  const nome = escapar(a.nome);
  if (a.tipo.startsWith("image/")) {
    return `<img class="anexo-img" data-anexo="${a.id}" src="${convertFileSrc(a.caminho)}" alt="${nome}" title="${nome}" />`;
  }
  return `<button type="button" class="anexo-chip" data-anexo="${a.id}" title="${nome}"><b>PDF</b> ${nome}</button>`;
}

/** Converte o texto do comentário (markdown + tokens de anexo) em HTML seguro. */
export function renderizar(texto: string, anexos: Map<number, Anexo>): string {
  const html = marked.parse(texto, { async: false });
  return html.replace(TOKEN_ANEXO, (_, id) => htmlAnexo(anexos.get(Number(id))));
}
