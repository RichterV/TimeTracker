<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, type Lista, type Tarefa } from "./api";
  import { estado } from "./estado.svelte";
  import { ArrowUpDown, ListIndentIncrease } from "@lucide/svelte";

  /** Posição "no fim" ao mover para dentro de um item. */
  const FIM = 1_000_000;

  const lerPref = (k: string, padrao: string) => {
    try {
      return localStorage.getItem(k) ?? padrao;
    } catch {
      return padrao;
    }
  };

  let listas = $state<Lista[]>([]);
  let listaId = $state<number | null>(Number(lerPref("tarefas.lista", "0")) || null);
  let tarefas = $state<Tarefa[]>([]);
  let ocultarFeitos = $state(lerPref("tarefas.ocultar", "0") === "1");
  let recolhidos = $state(new Set<number>());
  let renomeando = $state<number | null>(null);
  let erro = $state("");
  const campos = new Map<number, HTMLInputElement>();

  $effect(() => {
    try {
      if (listaId) localStorage.setItem("tarefas.lista", String(listaId));
      localStorage.setItem("tarefas.ocultar", ocultarFeitos ? "1" : "0");
    } catch {}
  });

  onMount(async () => {
    listas = await api.listarListas();
    if (!listas.some((l) => l.id === listaId)) listaId = listas[0]?.id ?? null;
  });

  $effect(() => {
    const id = listaId;
    if (id) api.listarTarefas(id).then((t) => (tarefas = t));
  });

  async function executar<T>(f: () => Promise<T>): Promise<T | undefined> {
    erro = "";
    try {
      return await f();
    } catch (e) {
      erro = String(e);
    }
  }

  // ---------- árvore ----------

  const porId = $derived(new Map(tarefas.map((t) => [t.id, t])));
  const filhosDe = $derived.by(() => {
    const m = new Map<number | null, Tarefa[]>();
    for (const t of tarefas) {
      const l = m.get(t.pai_id) ?? [];
      l.push(t);
      m.set(t.pai_id, l);
    }
    m.forEach((l) => l.sort((a, b) => a.ordem - b.ordem || a.id - b.id));
    return m;
  });

  type Linha = { t: Tarefa; nivel: number; total: number; feitos: number };

  const linhas = $derived.by(() => {
    const saida: Linha[] = [];
    const visitar = (pai: number | null, nivel: number) => {
      for (const t of filhosDe.get(pai) ?? []) {
        if (ocultarFeitos && t.feito) continue;
        const filhos = filhosDe.get(t.id) ?? [];
        saida.push({ t, nivel, total: filhos.length, feitos: filhos.filter((f) => f.feito).length });
        if (!recolhidos.has(t.id)) visitar(t.id, nivel + 1);
      }
    };
    visitar(null, 0);
    return saida;
  });

  const irmaosDe = (t: Tarefa) => filhosDe.get(t.pai_id) ?? [];

  function ehDescendente(id: number, ancestral: number) {
    for (let p = porId.get(id)?.pai_id ?? null; p !== null; p = porId.get(p)?.pai_id ?? null) {
      if (p === ancestral) return true;
    }
    return false;
  }

  function alternarRecolhido(id: number, recolher?: boolean) {
    const s = new Set(recolhidos);
    if (recolher ?? !s.has(id)) s.add(id);
    else s.delete(id);
    recolhidos = s;
  }

  async function focar(id: number, noFim = true) {
    await tick();
    const c = campos.get(id);
    if (!c) return;
    c.focus();
    const pos = noFim ? c.value.length : 0;
    c.setSelectionRange(pos, pos);
  }

  function registrar(no: HTMLInputElement, id: number) {
    campos.set(id, no);
    return { destroy: () => campos.get(id) === no && campos.delete(id) };
  }

  // ---------- operações ----------

  async function salvarTexto(t: Tarefa, texto: string) {
    if (texto === t.texto) return;
    t.texto = texto;
    await executar(() => api.editarTarefa(t.id, texto));
  }

  async function criar(paiId: number | null, apos: number | null) {
    if (!listaId) return;
    const r = await executar(() => api.criarTarefa(listaId!, paiId, apos));
    if (!r) return;
    tarefas = r.tarefas;
    if (paiId !== null) alternarRecolhido(paiId, false);
    focar(r.id);
  }

  async function marcar(t: Tarefa) {
    const r = await executar(() => api.marcarTarefa(listaId!, t.id, !t.feito));
    if (r) tarefas = r;
  }

  async function mover(id: number, novoPai: number | null, posicao: number) {
    const r = await executar(() => api.moverTarefa(listaId!, id, novoPai, posicao));
    if (r) tarefas = r;
    if (novoPai !== null) alternarRecolhido(novoPai, false);
  }

  async function excluir(t: Tarefa) {
    const r = await executar(() => api.excluirTarefa(listaId!, t.id));
    if (r) tarefas = r;
  }

  async function tecla(e: KeyboardEvent, l: Linha, i: number) {
    const t = l.t;
    const campo = e.currentTarget as HTMLInputElement;
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      await salvarTexto(t, campo.value);
      marcar(t);
    } else if (e.key === "Enter") {
      e.preventDefault();
      await salvarTexto(t, campo.value);
      criar(t.pai_id, t.id);
    } else if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      const irmaos = irmaosDe(t);
      const anterior = irmaos[irmaos.findIndex((x) => x.id === t.id) - 1];
      if (!anterior) return;
      await salvarTexto(t, campo.value);
      await mover(t.id, anterior.id, FIM);
      focar(t.id);
    } else if (e.key === "Tab" && e.shiftKey) {
      e.preventDefault();
      const pai = t.pai_id !== null ? porId.get(t.pai_id) : null;
      if (!pai) return;
      await salvarTexto(t, campo.value);
      const pos = irmaosDe(pai).findIndex((x) => x.id === pai.id) + 1;
      await mover(t.id, pai.pai_id, pos);
      focar(t.id);
    } else if (e.key === "Backspace" && campo.value === "") {
      e.preventDefault();
      const anterior = linhas[i - 1]?.t;
      await excluir(t);
      if (anterior) focar(anterior.id);
    } else if (e.key === "ArrowUp" && linhas[i - 1]) {
      e.preventDefault();
      focar(linhas[i - 1].t.id);
    } else if (e.key === "ArrowDown" && linhas[i + 1]) {
      e.preventDefault();
      focar(linhas[i + 1].t.id);
    } else if (e.key === "Escape") {
      campo.blur();
    }
  }

  // ---------- arrastar ----------

  type Zona = "antes" | "dentro" | "depois";
  /** reordenar: solta acima/abaixo no mesmo nível; aninhar: solta sobre o item e vira subitem */
  type ModoArraste = "reordenar" | "aninhar";
  type Arraste = { modo: ModoArraste; id: number; texto: string; x0: number; y0: number; x: number; y: number; moveu: boolean; alvo: number | null; zona: Zona | null };
  let arraste = $state<Arraste | null>(null);
  let lista: HTMLUListElement;

  function pegar(e: PointerEvent, t: Tarefa, modo: ModoArraste) {
    if (e.button !== 0) return;
    e.preventDefault();
    arraste = { modo, id: t.id, texto: t.texto || "(sem texto)", x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, moveu: false, alvo: null, zona: null };
  }

  function arrastar(e: PointerEvent) {
    const a = arraste;
    if (!a) return;
    a.x = e.clientX;
    a.y = e.clientY;
    if (!a.moveu && Math.hypot(e.clientX - a.x0, e.clientY - a.y0) < 4) return;
    a.moveu = true;
    a.alvo = null;
    a.zona = null;
    for (const li of lista.querySelectorAll<HTMLElement>("li[data-id]")) {
      const r = li.getBoundingClientRect();
      if (e.clientY < r.top || e.clientY >= r.bottom) continue;
      const id = Number(li.dataset.id);
      if (id === a.id || ehDescendente(id, a.id)) return;
      a.alvo = id;
      a.zona = a.modo === "aninhar" ? "dentro" : e.clientY < r.top + r.height / 2 ? "antes" : "depois";
      return;
    }
  }

  async function soltar() {
    const a = arraste;
    arraste = null;
    if (!a?.moveu || a.alvo === null || !a.zona) return;
    const alvo = porId.get(a.alvo)!;
    if (a.zona === "dentro") return mover(a.id, alvo.id, FIM);
    const irmaos = irmaosDe(alvo).filter((x) => x.id !== a.id);
    const pos = irmaos.findIndex((x) => x.id === alvo.id) + (a.zona === "depois" ? 1 : 0);
    mover(a.id, alvo.pai_id, pos);
  }

  // ---------- listas ----------

  async function novaLista() {
    const id = await executar(() => api.criarLista("Nova lista"));
    if (!id) return;
    listas = await api.listarListas();
    listaId = id;
    renomeando = id;
  }

  async function renomear(l: Lista, nome: string) {
    renomeando = null;
    nome = nome.trim();
    if (!nome || nome === l.nome) return;
    await executar(() => api.renomearLista(l.id, nome));
    listas = await api.listarListas();
  }

  async function excluirLista(l: Lista) {
    if (!(await estado.confirmar(`Excluir a lista "${l.nome}" e todos os itens?`, "Excluir"))) return;
    await executar(() => api.excluirLista(l.id));
    listas = await api.listarListas();
    if (listaId === l.id) listaId = listas[0]?.id ?? null;
  }

  function focarRenomear(no: HTMLInputElement) {
    no.focus();
    no.select();
  }

  const listaAtual = $derived(listas.find((l) => l.id === listaId));
  const pendentes = $derived(tarefas.filter((t) => !t.feito && !(filhosDe.get(t.id)?.length)).length);
</script>

<svelte:window onpointermove={arrastar} onpointerup={soltar} />

<div class="topo">
  <div class="listas">
    {#each listas as l (l.id)}
      {#if renomeando === l.id}
        <input
          class="renomear"
          value={l.nome}
          use:focarRenomear
          onblur={(e) => renomear(l, e.currentTarget.value)}
          onkeydown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            if (e.key === "Escape") renomeando = null;
          }}
        />
      {:else}
        <button
          class="lista"
          class:ativa={l.id === listaId}
          onclick={() => (listaId = l.id)}
          ondblclick={() => (renomeando = l.id)}
          title="Duplo clique para renomear"
        >
          {l.nome}
        </button>
      {/if}
    {/each}
    <button class="icone" onclick={novaLista} title="Nova lista">+ Lista</button>
  </div>
  <label class="secundario ocultar">
    <input type="checkbox" bind:checked={ocultarFeitos} /> Ocultar concluídos
  </label>
</div>

{#if erro}<p class="erro">{erro}</p>{/if}

<section class="cartao quadro" class:arrastando={arraste?.moveu}>
  <header>
    <h2>{listaAtual?.nome ?? ""}</h2>
    <span class="secundario">{pendentes} pendente{pendentes === 1 ? "" : "s"}</span>
    {#if listaAtual && listas.length > 1}
      <button class="icone perigo" title="Excluir lista" onclick={() => excluirLista(listaAtual)}>Excluir lista</button>
    {/if}
  </header>

  <ul bind:this={lista}>
    {#each linhas as l, i (l.t.id)}
      {@const t = l.t}
      <li
        data-id={t.id}
        class:feito={t.feito}
        class:origem={arraste?.moveu && arraste.id === t.id}
        class:alvo-antes={arraste?.alvo === t.id && arraste.zona === "antes"}
        class:alvo-depois={arraste?.alvo === t.id && arraste.zona === "depois"}
        class:alvo-dentro={arraste?.alvo === t.id && arraste.zona === "dentro"}
        style:--nivel={l.nivel}
      >
        <span class="alca" role="button" tabindex="-1" title="Arraste para reordenar" onpointerdown={(e) => pegar(e, t, "reordenar")}>
          <ArrowUpDown size={14} />
        </span>
        <span class="alca" role="button" tabindex="-1" title="Arraste sobre outro item para virar subitem" onpointerdown={(e) => pegar(e, t, "aninhar")}>
          <ListIndentIncrease size={14} />
        </span>
        {#if l.total > 0}
          <button class="seta" onclick={() => alternarRecolhido(t.id)} title={recolhidos.has(t.id) ? "Expandir" : "Recolher"}>
            {recolhidos.has(t.id) ? "▸" : "▾"}
          </button>
        {:else}
          <span class="seta"></span>
        {/if}
        <input type="checkbox" class="check" checked={t.feito} onchange={() => marcar(t)} />
        <input
          class="texto"
          value={t.texto}
          placeholder="Novo item"
          use:registrar={t.id}
          onkeydown={(e) => tecla(e, l, i)}
          onblur={(e) => salvarTexto(t, e.currentTarget.value)}
        />
        {#if l.total > 0}<span class="progresso secundario num">{l.feitos}/{l.total}</span>{/if}
        <button class="icone remover" title="Excluir item e subitens" onclick={() => excluir(t)}>✕</button>
      </li>
    {:else}
      <li class="vazio secundario">Lista vazia.</li>
    {/each}
  </ul>

  <button class="icone novo" onclick={() => criar(null, null)} disabled={!listaId}>+ Novo item</button>
  <p class="dica secundario">
    Enter novo item · Tab / Shift+Tab indenta · Ctrl+Enter marca · arraste ⇅ para reordenar ou ⇥ sobre um item para aninhar
  </p>
</section>

{#if arraste?.moveu}
  <div class="fantasma" style:left="{arraste.x + 12}px" style:top="{arraste.y + 8}px">
    <span class="modo">{arraste.modo === "aninhar" ? "Aninhar" : "Reordenar"}</span>
    {arraste.texto}
  </div>
{/if}

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    margin-bottom: 14px;
  }

  .listas {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .lista {
    border-radius: 16px;
    padding: 4px 14px;
  }

  .lista.ativa {
    background: var(--destaque);
    border-color: var(--destaque);
    color: var(--destaque-texto);
  }

  .renomear {
    width: 160px;
    border-radius: 16px;
    padding: 4px 12px;
  }

  .ocultar {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }

  .quadro {
    padding: 14px 16px;
    max-width: 820px;
  }

  .quadro.arrastando {
    cursor: grabbing;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  header .perigo {
    margin-left: auto;
    font-size: 12px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 1px 4px 1px calc(4px + var(--nivel) * 24px);
    border-radius: 6px;
    min-height: 32px;
  }

  li:hover {
    background: var(--superficie-2);
  }

  li.origem {
    opacity: 0.4;
  }

  li.alvo-antes::before,
  li.alvo-depois::after {
    content: "";
    position: absolute;
    left: calc(66px + var(--nivel) * 24px);
    right: 0;
    height: 2px;
    background: var(--destaque);
  }

  li.alvo-antes::before {
    top: -1px;
  }

  li.alvo-depois::after {
    bottom: -1px;
  }

  li.alvo-dentro {
    background: color-mix(in srgb, var(--destaque) 18%, transparent);
    outline: 1px dashed var(--destaque);
  }

  .alca {
    display: flex;
    align-items: center;
    cursor: grab;
    color: var(--texto-2);
    opacity: 0;
    padding: 3px;
    border-radius: 4px;
    touch-action: none;
  }

  .alca:hover {
    color: var(--texto);
    background: var(--borda);
  }

  li:hover .alca,
  li:focus-within .alca {
    opacity: 1;
  }

  .seta {
    width: 18px;
    flex-shrink: 0;
    background: none;
    border: none;
    padding: 0;
    color: var(--texto-2);
    font-size: 11px;
  }

  .check {
    width: 16px;
    height: 16px;
    margin: 0 4px 0 0;
    accent-color: var(--destaque);
    cursor: pointer;
    flex-shrink: 0;
  }

  .texto {
    flex: 1;
    border-color: transparent;
    background: transparent;
    padding: 4px 6px;
  }

  .texto:focus {
    background: var(--superficie);
  }

  li.feito .texto {
    text-decoration: line-through;
    color: var(--texto-2);
  }

  .progresso {
    font-size: 11px;
  }

  .remover {
    opacity: 0;
    font-size: 12px;
  }

  li:hover .remover {
    opacity: 1;
  }

  .vazio {
    padding: 16px 8px;
  }

  .novo {
    margin-top: 6px;
  }

  .dica {
    font-size: 11px;
    margin: 8px 0 0;
  }

  .fantasma {
    position: fixed;
    z-index: 30;
    pointer-events: none;
    background: var(--superficie);
    border: 1px solid var(--destaque);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 13px;
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.25);
    max-width: 280px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .fantasma .modo {
    font-size: 11px;
    color: var(--destaque);
    margin-right: 6px;
  }

  .erro {
    margin: 0 0 8px;
  }
</style>
