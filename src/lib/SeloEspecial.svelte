<script lang="ts">
  import type { DiaEspecial } from "./api";
  import { COR_ESPECIAL as CORES, ROTULO_ESPECIAL as ROTULO } from "./especiais";

  let { dia, compacto = false }: { dia: DiaEspecial | undefined; compacto?: boolean } = $props();
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
    line-height: 16px;
    padding: 1px 8px;
    border-radius: 10px;
    color: var(--cor);
    background: color-mix(in srgb, var(--cor) 15%, transparent);
  }

  .compacto {
    font-size: 10px;
    line-height: 15px;
    padding: 0 7px;
  }
</style>
