import type { Status } from "./api";

export const STATUS: { id: Status; nome: string; cor: string }[] = [
  { id: "nao_iniciada", nome: "Não iniciada", cor: "#8a93a3" },
  { id: "em_andamento", nome: "Em andamento", cor: "#3b6fe0" },
  { id: "rotina", nome: "Rotina", cor: "#8e5bd6" },
  { id: "concluida", nome: "Concluída", cor: "#2f9e5b" },
  { id: "finalizada", nome: "Finalizada", cor: "#6b7380" },
];

export const infoStatus = (s: Status) => STATUS.find((x) => x.id === s)!;

/** Concluída e Finalizada pedem confirmação antes de iniciar o timer. */
export const encerrada = (s: Status) => s === "concluida" || s === "finalizada";
