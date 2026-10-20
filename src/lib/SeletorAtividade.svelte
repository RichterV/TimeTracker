<script lang="ts">
  import { estado, projetoInativo } from "./estado.svelte";

  let { value = $bindable() }: { value: number | null } = $props();

  // inclui a atividade atual mesmo finalizada, para editar registros antigos
  const grupos = $derived(
    estado.projetos
      .map((p) => ({
        ...p,
        atividades: p.atividades.filter((a) => a.id === value || (a.status !== "finalizada" && !projetoInativo(p))),
      }))
      .filter((p) => p.atividades.length > 0),
  );
</script>

<select bind:value>
  <option value={null} disabled>Escolha a atividade</option>
  {#each grupos as p (p.id)}
    <optgroup label={p.nome}>
      {#each p.atividades as a (a.id)}
        <option value={a.id}>{a.nome}</option>
      {/each}
    </optgroup>
  {/each}
</select>
