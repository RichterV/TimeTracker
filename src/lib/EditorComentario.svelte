<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { tick } from "svelte";
  import { api, type Anexo } from "./api";
  import { tokenAnexo } from "./markdown";

  const EXTENSOES = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "pdf"];

  let {
    atividadeId,
    texto = $bindable(""),
    rotuloSalvar = "Comentar",
    autofoco = false,
    onsalvar,
    oncancelar,
    onanexos,
    onfoco,
    onerro,
  }: {
    atividadeId: number;
    texto?: string;
    rotuloSalvar?: string;
    autofoco?: boolean;
    onsalvar: (texto: string) => void;
    oncancelar?: () => void;
    onanexos: (novos: Anexo[]) => void;
    onfoco?: () => void;
    onerro: (msg: string) => void;
  } = $props();

  let campo: HTMLTextAreaElement;
  let enviando = $state(false);

  $effect(() => {
    if (autofoco) campo.focus();
  });

  /** Insere os tokens dos anexos na posição do cursor. */
  export async function inserirAnexos(anexos: Anexo[]) {
    if (anexos.length === 0) return;
    onanexos(anexos);
    const ini = campo.selectionStart ?? texto.length;
    const fim = campo.selectionEnd ?? texto.length;
    const antes = texto.slice(0, ini);
    const sep = antes && !/\s$/.test(antes) ? " " : "";
    const inserido = sep + anexos.map(tokenAnexo).join(" ") + " ";
    texto = antes + inserido + texto.slice(fim);
    await tick();
    campo.focus();
    campo.selectionStart = campo.selectionEnd = ini + inserido.length;
  }

  export async function anexarCaminhos(caminhos: string[]) {
    try {
      await inserirAnexos(await api.adicionarAnexos(atividadeId, caminhos));
    } catch (e) {
      onerro(String(e));
    }
  }

  export async function colarArquivo(arquivo: File) {
    const sub = arquivo.type.split("/")[1];
    const ext = sub === "jpeg" ? "jpg" : sub;
    const carimbo = new Date().toISOString().slice(0, 19).replace(/[-:]/g, "").replace("T", "_");
    try {
      const bytes = new Uint8Array(await arquivo.arrayBuffer());
      await inserirAnexos([await api.colarAnexo(atividadeId, `print_${carimbo}.${ext}`, bytes)]);
    } catch (e) {
      onerro(String(e));
    }
  }

  async function escolherArquivos() {
    const sel = await open({ multiple: true, filters: [{ name: "Imagens e PDF", extensions: EXTENSOES }] });
    if (sel) anexarCaminhos(Array.isArray(sel) ? sel : [sel]);
  }

  function colar(e: ClipboardEvent) {
    const arquivo = [...(e.clipboardData?.items ?? [])].find((i) => i.type.startsWith("image/"))?.getAsFile();
    if (!arquivo) return; // texto segue o comportamento normal
    e.preventDefault();
    e.stopPropagation();
    colarArquivo(arquivo);
  }

  function salvar() {
    if (!texto.trim() || enviando) return;
    onsalvar(texto);
  }

  function tecla(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      salvar();
    } else if (e.key === "Escape" && oncancelar) {
      e.stopPropagation();
      oncancelar();
    }
  }
</script>

<div class="editor">
  <textarea
    bind:this={campo}
    bind:value={texto}
    rows="3"
    placeholder="Escreva um comentário... (markdown; cole prints com Ctrl+V)"
    onpaste={colar}
    onkeydown={tecla}
    onfocus={() => onfoco?.()}
  ></textarea>
  <div class="barra">
    <button type="button" class="icone" onclick={escolherArquivos} title="Anexar imagem ou PDF">📎 Anexar</button>
    <span class="dica secundario">Ctrl+Enter para salvar</span>
    {#if oncancelar}<button type="button" onclick={oncancelar}>Cancelar</button>{/if}
    <button type="button" class="primario" disabled={!texto.trim()} onclick={salvar}>{rotuloSalvar}</button>
  </div>
</div>

<style>
  .editor {
    border: 1px solid var(--borda);
    border-radius: 8px;
    background: var(--superficie);
  }

  .editor:focus-within {
    border-color: var(--destaque);
  }

  textarea {
    width: 100%;
    border: none;
    background: transparent;
    resize: vertical;
    min-height: 64px;
    line-height: 1.5;
    padding: 8px 10px;
  }

  textarea:focus {
    outline: none;
  }

  .barra {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px 6px;
  }

  .dica {
    flex: 1;
    font-size: 11px;
  }

  .barra button {
    font-size: 13px;
    padding: 4px 10px;
  }
</style>
