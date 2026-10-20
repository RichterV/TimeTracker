import type { Registro } from "./api";

export const DIAS_CURTOS = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
export const DIA_SEG = 86400;

export const agoraSeg = () => Math.floor(Date.now() / 1000);
export const seg = (d: Date) => Math.floor(d.getTime() / 1000);

export function inicioDia(d: Date): Date {
  const x = new Date(d);
  x.setHours(0, 0, 0, 0);
  return x;
}

/** Segunda-feira 00:00 da semana de `d`. */
export function inicioSemana(d: Date): Date {
  const x = inicioDia(d);
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7));
  return x;
}

export function somarDias(d: Date, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

export function mesmoDia(a: Date, b: Date): boolean {
  return inicioDia(a).getTime() === inicioDia(b).getTime();
}

/** 1 = segunda ... 7 = domingo */
export const diaDaSemana = (d: Date) => ((d.getDay() + 6) % 7) + 1;

export function fmtDur(s: number): string {
  const m = Math.floor(Math.max(0, s) / 60);
  return `${Math.floor(m / 60)}h${String(m % 60).padStart(2, "0")}`;
}

export function fmtRelogio(s: number): string {
  s = Math.max(0, s);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return `${h}:${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
}

export const fmtHora = (ts: number) =>
  new Date(ts * 1000).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });

export const fmtDataLonga = (d: Date) =>
  d.toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "long" });

export const fmtDataCurta = (d: Date) =>
  d.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });

/** Segundos do registro que caem dentro de [de, ate). */
export function duracaoEm(r: Registro, de: number, ate: number, agora: number): number {
  const fim = Math.min(r.fim ?? agora, ate);
  return Math.max(0, fim - Math.max(r.inicio, de));
}

const pad = (n: number) => String(n).padStart(2, "0");

export function paraInputData(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function paraInputHora(ts: number): string {
  const d = new Date(ts * 1000);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Converte "AAAA-MM-DD" + "HH:MM" (hora local) para segundos unix. */
export function deInputs(data: string, hora: string): number {
  const [a, m, d] = data.split("-").map(Number);
  const [h, mi] = hora.split(":").map(Number);
  return seg(new Date(a, m - 1, d, h, mi));
}
