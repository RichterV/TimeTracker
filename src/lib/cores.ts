/** Paleta usada para novos projetos (alterna conforme a quantidade existente). */
export const CORES_PROJETO = ["#3b6fe0", "#2f9e5b", "#d1453b", "#e0912f", "#8e5bd6", "#1fa2a8", "#c94f8a", "#6b7380"];

export const corParaNovoProjeto = (quantidade: number) => CORES_PROJETO[quantidade % CORES_PROJETO.length];
