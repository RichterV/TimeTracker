<script lang="ts">
  import { api, type Tipo } from "./api";
  import { estado } from "./estado.svelte";
  import { ICONES } from "./icones";

  let escolhendoIcone = $state<number | null>(null);
  let erro = $state("");

  async function executar(f: () => Promise<unknown>) {
    erro = "";
    try {
      await f();
    } catch (e) {
      erro = String(e);
    }
    await estado.recarregarTipos();
  }

  const salvar = (t: Tipo, mudanca: Partial<Tipo>) => {
    const n = { ...t, ...mudanca };
    if (!n.nome.trim()) return;
    executar(() => api.editarTipo(t.id, n.nome, n.icone, n.cor));
  };

  async function excluir(t: Tipo) {
    if (await estado.confirmar(`Excluir o tipo "${t.nome}"?`, "Excluir")) executar(() => api.excluirTipo(t.id));
  }
</script>

<svelte:window onclick={() => (escolhendoIcone = null)} />

<section class="cartao">
  <h2>Tipos de atividade</h2>
  <ul>
    {#each estado.tipos as t (t.id)}
      {@const Icone = ICONES[t.icone] ?? ICONES.sparkles}
      <li>
        <div class="icone-wrap">
          <button
            type="button"
            class="icone-btn"
            title="Trocar ícone"
            onclick={(e) => {
              e.stopPropagation();
              escolhendoIcone = escolhendoIcone === t.id ? null : t.id;
            }}
          >
            <Icone size={18} color={t.cor} />
          </button>
          {#if escolhendoIcone === t.id}
            <div class="grade cartao" role="presentation" onclick={(e) => e.stopPropagation()}>
              {#each Object.entries(ICONES) as [nome, I]}
                <button
                  type="button"
                  class:ativo={nome === t.icone}
                  title={nome}
                  onclick={() => {
                    escolhendoIcone = null;
                    salvar(t, { icone: nome });
                  }}
                >
                  <I size={18} color={t.cor} />
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <input type="color" value={t.cor} title="Cor" onchange={(e) => salvar(t, { cor: e.currentTarget.value })} />
        <input class="nome" value={t.nome} onchange={(e) => salvar(t, { nome: e.currentTarget.value.trim() })} />
        <button type="button" class="icone perigo" title="Excluir tipo" onclick={() => excluir(t)}>✕</button>
      </li>
    {/each}
  </ul>
  <button type="button" onclick={() => executar(() => api.criarTipo("Novo tipo", "sparkles", "#3b6fe0"))}>+ Novo tipo</button>
  {#if erro}<p class="erro">{erro}</p>{/if}
</section>

<style>
  section {
    padding: 14px 16px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 15px;
    font-weight: 600;
  }

  ul {
    list-style: none;
    margin: 0 0 10px;
    padding: 0;
    display: grid;
    gap: 4px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .icone-wrap {
    position: relative;
  }

  .icone-btn {
    display: flex;
    padding: 5px;
  }

  .grade {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 20;
    display: grid;
    grid-template-columns: repeat(8, 32px);
    gap: 2px;
    padding: 6px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  .grade button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: none;
    border-color: transparent;
  }

  .grade button:hover,
  .grade button.ativo {
    background: var(--superficie-2);
    border-color: var(--borda);
  }

  input[type="color"] {
    width: 30px;
    height: 30px;
    padding: 2px;
    cursor: pointer;
  }

  .nome {
    flex: 1;
  }

  .erro {
    margin: 8px 0 0;
  }
</style>
