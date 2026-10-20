import { listen } from "@tauri-apps/api/event";
import { api, type Config, type Projeto, type Registro, type Status, type Tipo } from "./api";
import { encerrada, infoStatus } from "./status";
import { agoraSeg } from "./tempo";

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
  }

  async recarregarProjetos() {
    this.projetos = await api.listarProjetos();
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
