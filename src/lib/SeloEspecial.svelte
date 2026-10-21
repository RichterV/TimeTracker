<script lang="ts">
  import type { DiaEspecial } from "./api";

  let { dia, compacto = false }: { dia: DiaEspecial | undefined; compacto?: boolean } = $props();

  const CORES: Record<DiaEspecial["tipo"], string> = {
    feriado: "#d1453b",
    folga: "#1fa2a8",
    facultativo: "#e0912f",
    meio: "#e0912f",
  };
  const ROTULO: Record<DiaEspecial["tipo"], string> = {
    feriado: "Feriado",
    folga: "Folga",
    facultativo: "Ponto facultativo",
    meio: "Meio período",
  };
</script>

{#if dia}
  <span class="selo" class:compacto style:--cor={CORES[dia.tipo]} title="{ROTULO[dia.tipo]}: {dia.nome}">
    {dia.nome}
  </span>
{/if}

<style>
  .selo {
    display: inline-block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: middle;
    font-size: 11px;
    font-weight: 500;
    padding: 1px 8px;
    border-radius: 10px;
    color: var(--cor);
    background: color-mix(in srgb, var(--cor) 15%, transparent);
  }

  .compacto {
    font-size: 10px;
    padding: 0 6px;
  }
</style>
