# 8Pace

Tracking local de atividades (Tauri 2 + Svelte 5 + SQLite). Sem sincronismo: tudo fica em
`%APPDATA%\app.eightpace\8pace.db`.

## Rodar

```powershell
npm install
npm run tauri dev      # desenvolvimento
npm run tauri build    # instalador NSIS em src-tauri/target/release/bundle/nsis
```

Requisitos: Node 20+, Rust (rustup) e Visual Studio Build Tools com "Desenvolvimento para desktop com C++".

## Estrutura

- `src-tauri/src/db.rs`: schema, migrações e consultas SQLite
- `src-tauri/src/lib.rs`: comandos, bandeja, atalho global, ciclo de ociosidade e lembretes
- `src-tauri/src/ociosidade.rs`: tempo sem teclado/mouse (Windows)
- `src/lib/`: telas Svelte (Dia, Semana, Mês, Projetos, Configurações, seletor rápido)

Ícone: edite `src-tauri/icons/icone.svg` e rode `npx tauri icon src-tauri/icons/icone.svg`.
