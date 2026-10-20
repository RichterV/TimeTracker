import { invoke } from "@tauri-apps/api/core";

export type Status = "nao_iniciada" | "em_andamento" | "concluida" | "finalizada";

export type Atividade = {
  id: number;
  projeto_id: number;
  nome: string;
  status: Status;
  descricao: string;
  tipo_id: number | null;
};

export type Tipo = {
  id: number;
  nome: string;
  icone: string;
  cor: string;
  ordem: number;
};

export type Anexo = {
  id: number;
  atividade_id: number;
  nome: string;
  tipo: string;
  tamanho: number;
  criado: number;
  caminho: string;
  comentario_id: number | null;
};

export type Comentario = {
  id: number;
  atividade_id: number;
  texto: string;
  criado: number;
  editado: number | null;
};

export type Projeto = {
  id: number;
  nome: string;
  cor: string;
  arquivado: boolean;
  finalizado: boolean;
  atividades: Atividade[];
};

export type Registro = {
  id: number;
  atividade_id: number;
  atividade: string;
  projeto: string;
  cor: string;
  inicio: number;
  fim: number | null;
  nota: string;
};

export type RegistroEntrada = {
  id: number | null;
  atividade_id: number;
  inicio: number;
  fim: number | null;
  nota: string;
};

export type Intervalo = { inicio: string; fim: string };

export type Lista = { id: number; nome: string; ordem: number };

export type Tarefa = {
  id: number;
  lista_id: number;
  pai_id: number | null;
  texto: string;
  feito: boolean;
  ordem: number;
};

export type Config = {
  ociosidade_min: number;
  meta_diaria_min: number;
  dias_uteis: number[];
  expediente: Intervalo[];
  lembrete_ativo: boolean;
  lembrete_intervalo_min: number;
  atalho: string;
  iniciar_com_windows: boolean;
};

export type Ociosidade = { registro_id: number; inicio: number; fim: number };

export const api = {
  listarProjetos: () => invoke<Projeto[]>("listar_projetos"),
  criarProjeto: (nome: string, cor: string) => invoke<number>("criar_projeto", { nome, cor }),
  editarProjeto: (id: number, nome: string, cor: string, arquivado: boolean) =>
    invoke<void>("editar_projeto", { id, nome, cor, arquivado }),
  finalizarProjeto: (id: number, finalizado: boolean) =>
    invoke<void>("finalizar_projeto", { id, finalizado }),
  excluirProjeto: (id: number) => invoke<void>("excluir_projeto", { id }),
  criarAtividade: (projetoId: number, nome: string, tipoId: number) =>
    invoke<number>("criar_atividade", { projetoId, nome, tipoId }),
  definirTipo: (id: number, tipoId: number) => invoke<void>("definir_tipo", { id, tipoId }),

  listarTipos: () => invoke<Tipo[]>("listar_tipos"),
  criarTipo: (nome: string, icone: string, cor: string) => invoke<number>("criar_tipo", { nome, icone, cor }),
  editarTipo: (id: number, nome: string, icone: string, cor: string) =>
    invoke<void>("editar_tipo", { id, nome, icone, cor }),
  excluirTipo: (id: number) => invoke<void>("excluir_tipo", { id }),
  renomearAtividade: (id: number, nome: string) => invoke<void>("renomear_atividade", { id, nome }),

  listarComentarios: (atividadeId: number) => invoke<Comentario[]>("listar_comentarios", { atividadeId }),
  salvarComentario: (atividadeId: number, id: number | null, texto: string) =>
    invoke<Comentario>("salvar_comentario", { atividadeId, id, texto }),
  excluirComentario: (id: number) => invoke<void>("excluir_comentario", { id }),
  abrirLink: (url: string) => invoke<void>("abrir_link", { url }),

  listarListas: () => invoke<Lista[]>("listar_listas"),
  criarLista: (nome: string) => invoke<number>("criar_lista", { nome }),
  renomearLista: (id: number, nome: string) => invoke<void>("renomear_lista", { id, nome }),
  excluirLista: (id: number) => invoke<void>("excluir_lista", { id }),
  listarTarefas: (listaId: number) => invoke<Tarefa[]>("listar_tarefas", { listaId }),
  criarTarefa: (listaId: number, paiId: number | null, apos: number | null, texto = "") =>
    invoke<{ id: number; tarefas: Tarefa[] }>("criar_tarefa", { listaId, paiId, apos, texto }),
  editarTarefa: (id: number, texto: string) => invoke<void>("editar_tarefa", { id, texto }),
  marcarTarefa: (listaId: number, id: number, feito: boolean) =>
    invoke<Tarefa[]>("marcar_tarefa", { listaId, id, feito }),
  moverTarefa: (listaId: number, id: number, novoPai: number | null, posicao: number) =>
    invoke<Tarefa[]>("mover_tarefa", { listaId, id, novoPai, posicao }),
  excluirTarefa: (listaId: number, id: number) => invoke<Tarefa[]>("excluir_tarefa", { listaId, id }),
  definirStatus: (id: number, status: Status) => invoke<void>("definir_status", { id, status }),
  totalAtividade: (id: number) => invoke<number>("total_atividade", { id }),
  excluirAtividade: (id: number) => invoke<void>("excluir_atividade", { id }),

  listarAnexos: (atividadeId: number) => invoke<Anexo[]>("listar_anexos", { atividadeId }),
  adicionarAnexos: (atividadeId: number, caminhos: string[]) =>
    invoke<Anexo[]>("adicionar_anexos", { atividadeId, caminhos }),
  colarAnexo: (atividadeId: number, nome: string, bytes: Uint8Array) =>
    invoke<Anexo>("colar_anexo", bytes, { headers: { "atividade-id": String(atividadeId), nome } }),
  abrirAnexo: (id: number) => invoke<void>("abrir_anexo", { id }),
  excluirAnexo: (id: number) => invoke<void>("excluir_anexo", { id }),

  timerAtual: () => invoke<Registro | null>("timer_atual"),
  iniciar: (atividadeId: number) => invoke<void>("iniciar", { atividadeId }),
  parar: () => invoke<void>("parar"),

  listarRegistros: (de: number, ate: number) => invoke<Registro[]>("listar_registros", { de, ate }),
  salvarRegistro: (registro: RegistroEntrada) => invoke<void>("salvar_registro", { registro }),
  excluirRegistro: (id: number) => invoke<void>("excluir_registro", { id }),

  obterConfig: () => invoke<Config>("obter_config"),
  salvarConfig: (config: Config) => invoke<void>("salvar_config", { config }),
  pastaDados: () => invoke<string>("pasta_dados"),
  pastaTemBanco: (caminho: string) => invoke<boolean>("pasta_tem_banco", { caminho }),
  mudarPastaDados: (caminho: string, copiar: boolean) =>
    invoke<void>("mudar_pasta_dados", { caminho, copiar }),
  abrirPastaDados: () => invoke<void>("abrir_pasta_dados"),

  ociosidadePendente: () => invoke<Ociosidade | null>("ociosidade_pendente"),
  resolverOciosidade: (acao: "manter" | "descartar" | "atribuir", atividadeId?: number) =>
    invoke<void>("resolver_ociosidade", { acao, atividadeId: atividadeId ?? null }),

  mostrarSeletor: () => invoke<void>("mostrar_seletor"),
  fecharSeletor: () => invoke<void>("fechar_seletor"),
};
