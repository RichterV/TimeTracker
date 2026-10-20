<script lang="ts">
  import { estado } from "./estado.svelte";
  import { ICONES } from "./icones";

  let {
    atividadeId,
    tipoId,
    tamanho = 15,
  }: { atividadeId?: number; tipoId?: number | null; tamanho?: number } = $props();

  const tipo = $derived(
    estado.tipos.find(
      (t) => t.id === (tipoId ?? (atividadeId !== undefined ? estado.buscarAtividade(atividadeId)?.atividade.tipo_id : null)),
    ),
  );
  const Icone = $derived(tipo ? (ICONES[tipo.icone] ?? ICONES.sparkles) : null);
</script>

{#if tipo && Icone}
  <span class="icone-tipo" title={tipo.nome}>
    <Icone size={tamanho} color={tipo.cor} strokeWidth={2} />
  </span>
{/if}

<style>
  .icone-tipo {
    display: inline-flex;
    vertical-align: -2px;
    flex-shrink: 0;
  }
</style>
