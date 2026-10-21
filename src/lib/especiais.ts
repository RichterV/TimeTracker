import type { DiaEspecial } from "./api";

export const COR_ESPECIAL: Record<DiaEspecial["tipo"], string> = {
  feriado: "#d1453b",
  folga: "#1fa2a8",
  facultativo: "#e0912f",
  meio: "#e0912f",
};

export const ROTULO_ESPECIAL: Record<DiaEspecial["tipo"], string> = {
  feriado: "Feriado",
  folga: "Folga",
  facultativo: "Ponto facultativo",
  meio: "Meio período",
};
