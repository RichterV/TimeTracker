<script lang="ts">
  import "$lib/app.css";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { estado } from "$lib/estado.svelte";
  import Principal from "$lib/Principal.svelte";
  import Seletor from "$lib/Seletor.svelte";

  const rotulo = getCurrentWindow().label;
  let pronto = $state(false);

  onMount(async () => {
    await estado.iniciar();
    pronto = true;
  });
</script>

{#if pronto}
  {#if rotulo === "seletor"}
    <Seletor />
  {:else}
    <Principal />
  {/if}
{/if}
