import { listen } from "@tauri-apps/api/event";
import { api, type Config, type DiaEspecial, type Intervalo, type Projeto, type Registro, type Status, type Tipo } from "./api";
import { encerrada, infoStatus } from "./status";
import { agoraSeg, diaDaSemana, paraInputData } from "./tempo";

export type AtividadeItem = {
  id: number;
  nome: string;
  projeto: string;
  cor: string;
  status: Status;
  tipo_id: number | null;
};

/** Projeto arquivado ou finalizado não aparece para iniciar timer. */
export const projetoInativo = (p: Projeto) => p.arquivado || p.finalizado;

type Confirmacao = { mensagem: string; acao: string; responder: (ok: boolean) => void };

class Estado {
  projetos = $state<Projeto[]>([]);
  tipos = $state<Tipo[]>([]);
  rodando = $state<Registro | null>(null);
  config = $state<Config | null>(null);
  agora = $state(agoraSeg());
  /** Incrementado a cada mudança de registros, para as telas recarregarem. */
  versao = $state(0);
  /** Atividade exibida no painel de detalhes. */
  atividadeAberta = $state<number | null>(null);
  confirmacao = $state<Confirmacao | null>(null);
  /** Feriados, facultativos e folgas por data (AAAA-MM-DD), dos anos já carregados. */
  especiais = $state(new Map<string, DiaEspecial>());
  #anos = new Set<number>();

  /** Atividades disponíveis para iniciar (projeto ativo, atividade não finalizada). */
  atividades = $derived<AtividadeItem[]>(
    this.projetos
      .filter((p) => !projetoInativo(p))
      .flatMap((p) =>
        p.atividades
          .filter((a) => a.status !== "finalizada")
          .map((a) => ({ id: a.id, nome: a.nome, projeto: p.nome, cor: p.cor, status: a.status, tipo_id: a.tipo_id })),
      ),
  );

  /** Tipo sugerido para atividades novas: Feature, ou o primeiro da lista. */
  tipoPadrao = $derived(this.tipos.find((t) => t.nome === "Feature")?.id ?? this.tipos[0]?.id ?? null);

  #iniciado = false;

  async iniciar() {
    if (this.#iniciado) return;
    this.#iniciado = true;
    setInterval(() => (this.agora = agoraSeg()), 1000);
    await listen("timer-mudou", () => {
      this.recarregarEspeciais();
      this.recarregarTimer();
      this.recarregarProjetos();
      this.recarregarTipos();
      this.versao++;
    });
    await Promise.all([
      this.recarregarProjetos(),
      this.recarregarTipos(),
      this.recarregarTimer(),
      api.obterConfig().then((c) => (this.config = c)),
    ]);
    const ano = new Date().getFullYear();
    await this.garantirAnos(ano - 1, ano + 1);
  }

  async recarregarProjetos() {
    this.projetos = await api.listarProjetos();
  }

  /** Carrega os dias especiais dos anos indicados (se ainda não carregados). */
  async garantirAnos(...anos: number[]) {
    const faltam = anos.filter((a) => !this.#anos.has(a));
    if (faltam.length === 0) return;
    faltam.forEach((a) => this.#anos.add(a));
    const de = `${Math.min(...faltam)}-01-01`;
    const ate = `${Math.max(...faltam)}-12-31`;
    const lista = await api.diasEspeciais(de, ate);
    const m = new Map(this.especiais);
    lista.forEach((d) => m.set(d.data, d));
    this.especiais = m;
  }

  /** Recarrega tudo (após mudar folgas ou facultativos). */
  async recarregarEspeciais() {
    if (this.#anos.size === 0) return;
    const anos = [...this.#anos];
    const lista = await api.diasEspeciais(`${Math.min(...anos)}-01-01`, `${Math.max(...anos)}-12-31`);
    this.especiais = new Map(lista.map((d) => [d.data, d]));
  }

  especial(d: Date): DiaEspecial | undefined {
    return this.especiais.get(paraInputData(d));
  }

  /** Meta do dia em segundos: zero fora dos dias úteis, em feriados e folgas; metade em meio período. */
  metaDoDia(d: Date): number {
    if (!this.config || !this.config.dias_uteis.includes(diaDaSemana(d))) return 0;
    const meta = this.config.meta_diaria_min * 60;
    const e = this.especial(d);
    if (!e) return meta;
    return e.tipo === "meio" ? Math.round(meta / 2) : 0;
  }

  /** Intervalos de expediente do dia, já considerando feriados, folgas e meio período. */
  expedienteDoDia(d: Date): Intervalo[] {
    if (!this.config || !this.config.dias_uteis.includes(diaDaSemana(d))) return [];
    const e = this.especial(d);
    if (!e) return this.config.expediente;
    if (e.tipo !== "meio" || !e.a_partir) return [];
    const corte = e.a_partir;
    return this.config.expediente
      .filter((i) => i.fim > corte)
      .map((i) => ({ inicio: i.inicio < corte ? corte : i.inicio, fim: i.fim }));
  }

  async recarregarTipos() {
    this.tipos = await api.listarTipos();
  }

  async recarregarTimer() {
    this.rodando = await api.timerAtual();
  }

  buscarAtividade(id: number) {
    for (const p of this.projetos) {
      const a = p.atividades.find((a) => a.id === id);
      if (a) return { atividade: a, projeto: p };
    }
    return null;
  }

  confirmar(mensagem: string, acao: string): Promise<boolean> {
    return new Promise((resolve) => {
      this.confirmacao = {
        mensagem,
        acao,
        responder: (ok) => {
          this.confirmacao = null;
          resolve(ok);
        },
      };
    });
  }

  /** Inicia o timer, pedindo confirmação se a atividade estiver Concluída ou Finalizada. */
  async iniciarAtividade(id: number) {
    const a = this.buscarAtividade(id)?.atividade;
    if (a && encerrada(a.status)) {
      const ok = await this.confirmar(
        `"${a.nome}" está ${infoStatus(a.status).nome.toLowerCase()}. Iniciar o timer volta a atividade para Em andamento.`,
        "Iniciar mesmo assim",
      );
      if (!ok) return;
    }
    await api.iniciar(id);
  }
}

export const estado = new Estado();
