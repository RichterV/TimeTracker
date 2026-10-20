<script lang="ts">
  import { estado } from "./estado.svelte";
  import Modal from "./Modal.svelte";

  const c = $derived(estado.confirmacao!);
</script>

<Modal titulo="Confirmar" onfechar={() => c.responder(false)}>
  <p>{c.mensagem}</p>
  <div class="acoes">
    <button onclick={() => c.responder(false)}>Cancelar</button>
    <button class="primario" onclick={() => c.responder(true)}>{c.acao}</button>
  </div>
</Modal>

<style>
  p {
    margin: 0;
    line-height: 1.5;
  }

  .acoes {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
