mod db;
mod feriados;
mod ociosidade;
mod tarefas;

use chrono::{Datelike, Local, TimeZone};
use db::{Config, Projeto, Registro, RegistroEntrada};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

#[derive(Serialize, Clone)]
struct Ociosidade {
    registro_id: i64,
    inicio: i64,
    fim: i64,
}

#[derive(Default)]
struct EstadoOcio {
    /// Início do período ocioso em andamento (timer rodando e usuário ausente).
    inicio: Option<i64>,
    /// Período já encerrado aguardando decisão do usuário.
    pendente: Option<Ociosidade>,
    ultimo_lembrete: i64,
    ultimo_ciclo: i64,
}

struct Estado {
    /// Pasta com o banco e os anexos (escolhida pelo usuário).
    pasta: PathBuf,
    db: Mutex<Connection>,
    config: Mutex<Config>,
    ocio: Mutex<EstadoOcio>,
}

struct ItensTray {
    status: MenuItem<Wry>,
    alternar: MenuItem<Wry>,
}

fn agora() -> i64 {
    chrono::Utc::now().timestamp()
}

fn erro<E: ToString>(e: E) -> String {
    e.to_string()
}

fn inicio_de_hoje() -> i64 {
    let hoje = Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    Local.from_local_datetime(&hoje).earliest().unwrap().timestamp()
}

fn fmt_duracao(seg: i64) -> String {
    let min = seg.max(0) / 60;
    format!("{}h{:02}", min / 60, min % 60)
}

// ---------- janelas e bandeja ----------

fn mostrar_janela(app: &AppHandle, rotulo: &str) {
    if let Some(w) = app.get_webview_window(rotulo) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn abrir_seletor(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("seletor") {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit_to("seletor", "seletor-abrir", ());
    }
}

fn atualizar_tray(app: &AppHandle) {
    let estado = app.state::<Estado>();
    let (rodando, hoje) = {
        let conn = estado.db.lock().unwrap();
        let t = agora();
        let de = inicio_de_hoje();
        (
            db::registro_rodando(&conn).ok().flatten(),
            db::total_periodo(&conn, de, de + 86400, t).unwrap_or(0),
        )
    };
    let meta = estado.config.lock().unwrap().meta_diaria_min as i64 * 60;
    let itens = app.state::<ItensTray>();
    let (status, dica) = match &rodando {
        Some(r) => {
            let txt = format!("{} ({}) {}", r.atividade, r.projeto, fmt_duracao(agora() - r.inicio));
            (txt.clone(), txt)
        }
        None => ("Nenhum timer rodando".to_string(), "Nenhum timer rodando".to_string()),
    };
    let _ = itens.status.set_text(&status);
    let _ = itens
        .alternar
        .set_text(if rodando.is_some() { "Parar timer" } else { "Iniciar atividade..." });
    if let Some(tray) = app.tray_by_id("principal") {
        let dica = format!("8Pace\n{dica}\nHoje: {} / {}", fmt_duracao(hoje), fmt_duracao(meta));
        let _ = tray.set_tooltip(Some(dica));
    }
}

fn timer_mudou(app: &AppHandle) {
    atualizar_tray(app);
    let _ = app.emit("timer-mudou", ());
}

fn criar_tray(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Nenhum timer rodando", false, None::<&str>)?;
    let alternar = MenuItem::with_id(app, "alternar", "Iniciar atividade...", true, None::<&str>)?;
    let abrir = MenuItem::with_id(app, "abrir", "Abrir 8Pace", true, None::<&str>)?;
    let sair = MenuItem::with_id(app, "sair", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &alternar,
            &abrir,
            &PredefinedMenuItem::separator(app)?,
            &sair,
        ],
    )?;
    app.manage(ItensTray { status, alternar });

    TrayIconBuilder::with_id("principal")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("8Pace")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "alternar" => {
                let rodando = {
                    let estado = app.state::<Estado>();
                    let conn = estado.db.lock().unwrap();
                    db::registro_rodando(&conn).ok().flatten().is_some()
                };
                if rodando {
                    let _ = parar(app.clone(), app.state());
                } else {
                    abrir_seletor(app);
                }
            }
            "abrir" => mostrar_janela(app, "main"),
            "sair" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                mostrar_janela(tray.app_handle(), "main");
            }
        })
        .build(app)?;
    Ok(())
}

// ---------- ciclo de fundo: ociosidade, lembrete, bandeja ----------

fn no_expediente(conn: &Connection, cfg: &Config) -> bool {
    let agora = Local::now();
    if !cfg.dias_uteis.contains(&agora.weekday().number_from_monday()) {
        return false;
    }
    let hoje = agora.date_naive();
    let hora = agora.format("%H:%M").to_string();
    // feriado, folga ou facultativo: sem expediente; meio período só depois do horário indicado
    if let Some(especial) = feriados::no_periodo(conn, hoje, hoje, &cfg.facultativos).ok().and_then(|v| v.into_iter().next()) {
        match especial.a_partir {
            Some(a_partir) if hora >= a_partir => {}
            _ => return false,
        }
    }
    cfg.expediente
        .iter()
        .any(|i| hora.as_str() >= i.inicio.as_str() && hora.as_str() < i.fim.as_str())
}

fn ciclo(app: &AppHandle, contador: u64) {
    let estado = app.state::<Estado>();
    let cfg = estado.config.lock().unwrap().clone();
    let t = agora();
    let ocioso = ociosidade::segundos_ocioso();
    let limite = cfg.ociosidade_min as i64 * 60;
    let (rodando, expediente) = {
        let conn = estado.db.lock().unwrap();
        let _ = db::registrar_batimento(&conn, t);
        (db::registro_rodando(&conn).ok().flatten(), no_expediente(&conn, &cfg))
    };

    let mut avisar_ocio = false;
    let mut lembrar = false;
    {
        let mut o = estado.ocio.lock().unwrap();
        match &rodando {
            None => {
                o.inicio = None;
                if o.pendente.take().is_some() {
                    let _ = app.emit("ociosidade", None::<Ociosidade>);
                }
            }
            Some(r) => {
                if o.pendente.as_ref().is_some_and(|p| p.registro_id != r.id) {
                    o.pendente = None;
                    let _ = app.emit("ociosidade", None::<Ociosidade>);
                }
                if o.pendente.is_none() && limite > 0 {
                    // salto grande entre ciclos = PC suspenso
                    let suspenso = o.ultimo_ciclo > 0 && t - o.ultimo_ciclo > limite.max(60);
                    if o.inicio.is_none() {
                        if suspenso {
                            o.inicio = Some(o.ultimo_ciclo.max(r.inicio));
                        } else if ocioso >= limite {
                            o.inicio = Some((t - ocioso).max(r.inicio));
                        }
                    }
                    if let Some(ini) = o.inicio {
                        if ocioso < 10 {
                            o.inicio = None;
                            let fim = t - ocioso;
                            if fim - ini >= limite {
                                o.pendente = Some(Ociosidade { registro_id: r.id, inicio: ini, fim });
                                avisar_ocio = true;
                            }
                        }
                    }
                }
            }
        }

        if rodando.is_none()
            && cfg.lembrete_ativo
            && ocioso < 300
            && expediente
            && t - o.ultimo_lembrete >= cfg.lembrete_intervalo_min.max(1) as i64 * 60
        {
            o.ultimo_lembrete = t;
            lembrar = true;
        }
        o.ultimo_ciclo = t;

        if avisar_ocio {
            let _ = app.emit("ociosidade", o.pendente.clone());
        }
    }

    if avisar_ocio {
        mostrar_janela(app, "main");
    }
    if lembrar {
        let _ = app
            .notification()
            .builder()
            .title("8Pace")
            .body("Nenhum timer rodando. Ctrl+Alt+Space para iniciar uma atividade.")
            .show();
    }
    if contador % 6 == 0 {
        atualizar_tray(app);
    }
}

/// Se o app parou de rodar (PC desligado, travou, foi encerrado) com o timer ligado,
/// fecha o registro no último batimento. Devolve a atividade e o horário da pausa.
fn pausar_apos_desligamento(conn: &Connection) -> rusqlite::Result<Option<(String, i64)>> {
    let (Some(r), Some(batimento)) = (db::registro_rodando(conn)?, db::ultimo_batimento(conn)?) else {
        return Ok(None);
    };
    if agora() - batimento < 60 {
        return Ok(None);
    }
    if batimento <= r.inicio {
        db::excluir_registro(conn, r.id)?;
    } else {
        db::parar(conn, batimento)?;
    }
    Ok(Some((r.atividade, batimento)))
}

// ---------- comandos ----------

#[tauri::command]
fn listar_projetos(estado: State<Estado>) -> Res<Vec<Projeto>> {
    db::listar_projetos(&estado.db.lock().unwrap()).map_err(erro)
}

#[tauri::command]
fn criar_projeto(estado: State<Estado>, nome: String, cor: String) -> Res<i64> {
    db::criar_projeto(&estado.db.lock().unwrap(), nome.trim(), &cor).map_err(erro)
}

#[tauri::command]
fn editar_projeto(app: AppHandle, estado: State<Estado>, id: i64, nome: String, cor: String, arquivado: bool) -> Res<()> {
    db::editar_projeto(&estado.db.lock().unwrap(), id, nome.trim(), &cor, arquivado).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn finalizar_projeto(app: AppHandle, estado: State<Estado>, id: i64, finalizado: bool) -> Res<()> {
    db::finalizar_projeto(&estado.db.lock().unwrap(), id, finalizado).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn excluir_projeto(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    let conn = estado.db.lock().unwrap();
    let atividades = db::atividades_do_projeto(&conn, id).map_err(erro)?;
    if !db::excluir_projeto(&conn, id).map_err(erro)? {
        return Err("O projeto tem registros de tempo. Arquive em vez de excluir.".into());
    }
    for a in atividades {
        remover_pasta_anexos(&app, a);
    }
    Ok(())
}

#[tauri::command]
fn criar_atividade(estado: State<Estado>, projeto_id: i64, nome: String, tipo_id: i64) -> Res<i64> {
    db::criar_atividade(&estado.db.lock().unwrap(), projeto_id, nome.trim(), tipo_id).map_err(erro)
}

#[tauri::command]
fn definir_tipo(app: AppHandle, estado: State<Estado>, id: i64, tipo_id: i64) -> Res<()> {
    db::definir_tipo(&estado.db.lock().unwrap(), id, tipo_id).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn listar_tipos(estado: State<Estado>) -> Res<Vec<db::Tipo>> {
    db::listar_tipos(&estado.db.lock().unwrap()).map_err(erro)
}

#[tauri::command]
fn criar_tipo(app: AppHandle, estado: State<Estado>, nome: String, icone: String, cor: String) -> Res<i64> {
    let id = db::criar_tipo(&estado.db.lock().unwrap(), nome.trim(), &icone, &cor).map_err(erro)?;
    timer_mudou(&app);
    Ok(id)
}

#[tauri::command]
fn editar_tipo(app: AppHandle, estado: State<Estado>, id: i64, nome: String, icone: String, cor: String) -> Res<()> {
    db::editar_tipo(&estado.db.lock().unwrap(), id, nome.trim(), &icone, &cor).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn excluir_tipo(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    let usados = db::excluir_tipo(&estado.db.lock().unwrap(), id).map_err(erro)?;
    if usados > 0 {
        return Err(format!("{usados} atividade(s) usam este tipo. Troque o tipo delas antes de excluir."));
    }
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn renomear_atividade(app: AppHandle, estado: State<Estado>, id: i64, nome: String) -> Res<()> {
    db::renomear_atividade(&estado.db.lock().unwrap(), id, nome.trim()).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn listar_comentarios(estado: State<Estado>, atividade_id: i64) -> Res<Vec<db::Comentario>> {
    db::listar_comentarios(&estado.db.lock().unwrap(), atividade_id).map_err(erro)
}

#[tauri::command]
fn salvar_comentario(estado: State<Estado>, atividade_id: i64, id: Option<i64>, texto: String) -> Res<db::Comentario> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Err("O comentário está vazio.".into());
    }
    db::salvar_comentario(&mut estado.db.lock().unwrap(), atividade_id, id, texto, agora()).map_err(erro)
}

#[tauri::command]
fn excluir_comentario(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    let anexos = db::excluir_comentario(&mut estado.db.lock().unwrap(), id).map_err(erro)?;
    let pasta = pasta_anexos(&app)?;
    for a in anexos {
        let _ = std::fs::remove_file(pasta.join(&a.arquivo));
    }
    Ok(())
}

// ---------- relatórios ----------

/// Grava o PDF gerado na tela no caminho escolhido na janela de salvar e, se pedido, abre.
#[tauri::command]
fn gravar_pdf(app: AppHandle, caminho: String, bytes: Vec<u8>, abrir: bool) -> Res<()> {
    if !caminho.to_lowercase().ends_with(".pdf") {
        return Err("O arquivo precisa ter extensão .pdf.".into());
    }
    std::fs::write(&caminho, bytes).map_err(erro)?;
    if abrir {
        app.opener().open_path(caminho, None::<&str>).map_err(erro)?;
    }
    Ok(())
}

// ---------- feriados e folgas ----------

fn data_param(s: &str) -> Res<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| format!("Data inválida: {s}"))
}

/// Feriados, facultativos e folgas entre `de` e `ate` (AAAA-MM-DD, inclusivo).
#[tauri::command]
fn dias_especiais(estado: State<Estado>, de: String, ate: String) -> Res<Vec<feriados::DiaEspecial>> {
    let facultativos = estado.config.lock().unwrap().facultativos.clone();
    feriados::no_periodo(&estado.db.lock().unwrap(), data_param(&de)?, data_param(&ate)?, &facultativos).map_err(erro)
}

#[tauri::command]
fn listar_folgas(estado: State<Estado>) -> Res<Vec<feriados::Folga>> {
    feriados::listar_folgas(&estado.db.lock().unwrap()).map_err(erro)
}

#[tauri::command]
fn criar_folga(app: AppHandle, estado: State<Estado>, inicio: String, fim: String, descricao: String) -> Res<i64> {
    if data_param(&fim)? < data_param(&inicio)? {
        return Err("O fim precisa ser igual ou depois do início.".into());
    }
    let id = feriados::criar_folga(&estado.db.lock().unwrap(), &inicio, &fim, descricao.trim()).map_err(erro)?;
    timer_mudou(&app);
    Ok(id)
}

#[tauri::command]
fn excluir_folga(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    feriados::excluir_folga(&estado.db.lock().unwrap(), id).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

// ---------- tarefas (todo) ----------

#[tauri::command]
fn listar_listas(estado: State<Estado>) -> Res<Vec<tarefas::Lista>> {
    tarefas::listar_listas(&estado.db.lock().unwrap()).map_err(erro)
}

#[tauri::command]
fn criar_lista(estado: State<Estado>, nome: String) -> Res<i64> {
    tarefas::criar_lista(&estado.db.lock().unwrap(), nome.trim()).map_err(erro)
}

#[tauri::command]
fn renomear_lista(estado: State<Estado>, id: i64, nome: String) -> Res<()> {
    tarefas::renomear_lista(&estado.db.lock().unwrap(), id, nome.trim()).map_err(erro)
}

#[tauri::command]
fn excluir_lista(estado: State<Estado>, id: i64) -> Res<()> {
    let conn = estado.db.lock().unwrap();
    if tarefas::listar_listas(&conn).map_err(erro)?.len() <= 1 {
        return Err("Mantenha pelo menos uma lista.".into());
    }
    tarefas::excluir_lista(&conn, id).map_err(erro)
}

#[tauri::command]
fn listar_tarefas(estado: State<Estado>, lista_id: i64) -> Res<Vec<tarefas::Tarefa>> {
    tarefas::listar_tarefas(&estado.db.lock().unwrap(), lista_id).map_err(erro)
}

#[derive(Serialize)]
struct TarefaCriada {
    id: i64,
    tarefas: Vec<tarefas::Tarefa>,
}

#[tauri::command]
fn criar_tarefa(
    estado: State<Estado>,
    lista_id: i64,
    pai_id: Option<i64>,
    apos: Option<i64>,
    texto: String,
) -> Res<TarefaCriada> {
    let mut conn = estado.db.lock().unwrap();
    let id = tarefas::criar_tarefa(&mut conn, lista_id, pai_id, apos, &texto, agora()).map_err(erro)?;
    Ok(TarefaCriada { id, tarefas: tarefas::listar_tarefas(&conn, lista_id).map_err(erro)? })
}

#[tauri::command]
fn editar_tarefa(estado: State<Estado>, id: i64, texto: String) -> Res<()> {
    tarefas::editar_tarefa(&estado.db.lock().unwrap(), id, &texto).map_err(erro)
}

#[tauri::command]
fn marcar_tarefa(estado: State<Estado>, lista_id: i64, id: i64, feito: bool) -> Res<Vec<tarefas::Tarefa>> {
    let mut conn = estado.db.lock().unwrap();
    tarefas::marcar_tarefa(&mut conn, id, feito).map_err(erro)?;
    tarefas::listar_tarefas(&conn, lista_id).map_err(erro)
}

#[tauri::command]
fn mover_tarefa(
    estado: State<Estado>,
    lista_id: i64,
    id: i64,
    novo_pai: Option<i64>,
    posicao: usize,
) -> Res<Vec<tarefas::Tarefa>> {
    let mut conn = estado.db.lock().unwrap();
    if !tarefas::mover_tarefa(&mut conn, id, novo_pai, posicao).map_err(erro)? {
        return Err("Não dá para mover um item para dentro dele mesmo.".into());
    }
    tarefas::listar_tarefas(&conn, lista_id).map_err(erro)
}

#[tauri::command]
fn excluir_tarefa(estado: State<Estado>, lista_id: i64, id: i64) -> Res<Vec<tarefas::Tarefa>> {
    let mut conn = estado.db.lock().unwrap();
    tarefas::excluir_tarefa(&mut conn, id).map_err(erro)?;
    tarefas::listar_tarefas(&conn, lista_id).map_err(erro)
}

/// Abre um link dos comentários no navegador padrão.
#[tauri::command]
fn abrir_link(app: AppHandle, url: String) -> Res<()> {
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")) {
        return Err("Link não suportado.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(erro)
}

#[tauri::command]
fn definir_status(app: AppHandle, estado: State<Estado>, id: i64, status: String) -> Res<()> {
    if !db::STATUS.contains(&status.as_str()) {
        return Err(format!("Status inválido: {status}"));
    }
    db::definir_status(&estado.db.lock().unwrap(), id, &status).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn total_atividade(estado: State<Estado>, id: i64) -> Res<i64> {
    db::total_atividade(&estado.db.lock().unwrap(), id, agora()).map_err(erro)
}

#[tauri::command]
fn excluir_atividade(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    if !db::excluir_atividade(&estado.db.lock().unwrap(), id).map_err(erro)? {
        return Err("A atividade tem registros de tempo. Marque como Finalizada em vez de excluir.".into());
    }
    remover_pasta_anexos(&app, id);
    Ok(())
}

// ---------- anexos ----------

fn pasta_anexos(app: &AppHandle) -> Res<PathBuf> {
    Ok(app.state::<Estado>().pasta.join("anexos"))
}

fn remover_pasta_anexos(app: &AppHandle, atividade_id: i64) {
    if let Ok(dir) = pasta_anexos(app) {
        let _ = std::fs::remove_dir_all(dir.join(atividade_id.to_string()));
    }
}

fn tipo_por_extensao(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "pdf" => "application/pdf",
        _ => return None,
    })
}

fn com_caminho(app: &AppHandle, mut a: db::Anexo) -> Res<db::Anexo> {
    a.caminho = pasta_anexos(app)?.join(&a.arquivo).display().to_string();
    Ok(a)
}

/// Registra o anexo e grava o arquivo em anexos/<atividade>/<id>.<ext> usando `escrever`.
fn gravar_anexo(
    app: &AppHandle,
    estado: &Estado,
    atividade_id: i64,
    nome: &str,
    escrever: impl FnOnce(&Path) -> std::io::Result<u64>,
) -> Res<db::Anexo> {
    let ext = Path::new(nome)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let tipo = tipo_por_extensao(&ext)
        .ok_or_else(|| format!("{nome}: só imagens (png, jpg, gif, webp, bmp) ou PDF."))?;
    let dir = pasta_anexos(app)?.join(atividade_id.to_string());
    std::fs::create_dir_all(&dir).map_err(erro)?;

    let conn = estado.db.lock().unwrap();
    let id = db::inserir_anexo(&conn, atividade_id, nome, tipo, agora()).map_err(erro)?;
    let arquivo = format!("{atividade_id}/{id}.{ext}");
    match escrever(&dir.join(format!("{id}.{ext}"))) {
        Ok(tamanho) => db::concluir_anexo(&conn, id, &arquivo, tamanho as i64).map_err(erro)?,
        Err(e) => {
            let _ = db::excluir_anexo(&conn, id);
            return Err(format!("{nome}: {e}"));
        }
    }
    let anexo = db::obter_anexo(&conn, id).map_err(erro)?.ok_or("Anexo não encontrado.")?;
    com_caminho(app, anexo)
}

#[tauri::command]
fn listar_anexos(app: AppHandle, estado: State<Estado>, atividade_id: i64) -> Res<Vec<db::Anexo>> {
    let anexos = db::listar_anexos(&estado.db.lock().unwrap(), atividade_id).map_err(erro)?;
    anexos.into_iter().map(|a| com_caminho(&app, a)).collect()
}

/// Copia arquivos do disco (seletor de arquivos ou arrastar e soltar).
#[tauri::command]
fn adicionar_anexos(app: AppHandle, estado: State<Estado>, atividade_id: i64, caminhos: Vec<String>) -> Res<Vec<db::Anexo>> {
    caminhos
        .iter()
        .map(|c| {
            let origem = Path::new(c);
            let nome = origem.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            gravar_anexo(&app, &estado, atividade_id, &nome, |destino| std::fs::copy(origem, destino))
        })
        .collect()
}

/// Recebe os bytes crus de uma imagem colada. Cabeçalhos: `atividade-id` e `nome`.
#[tauri::command]
fn colar_anexo(app: AppHandle, estado: State<Estado>, request: tauri::ipc::Request) -> Res<db::Anexo> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("Conteúdo colado inválido.".into());
    };
    let cabecalho = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(str::to_string);
    let atividade_id: i64 = cabecalho("atividade-id")
        .and_then(|v| v.parse().ok())
        .ok_or("Atividade não informada.")?;
    let nome = cabecalho("nome").unwrap_or_else(|| "print.png".into());
    gravar_anexo(&app, &estado, atividade_id, &nome, |destino| {
        std::fs::write(destino, bytes).map(|_| bytes.len() as u64)
    })
}

#[tauri::command]
fn abrir_anexo(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    let anexo = db::obter_anexo(&estado.db.lock().unwrap(), id).map_err(erro)?.ok_or("Anexo não encontrado.")?;
    let anexo = com_caminho(&app, anexo)?;
    app.opener().open_path(anexo.caminho, None::<&str>).map_err(erro)
}

#[tauri::command]
fn excluir_anexo(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    let conn = estado.db.lock().unwrap();
    if let Some(a) = db::obter_anexo(&conn, id).map_err(erro)? {
        let _ = std::fs::remove_file(pasta_anexos(&app)?.join(&a.arquivo));
        db::excluir_anexo(&conn, id).map_err(erro)?;
    }
    Ok(())
}

#[tauri::command]
fn timer_atual(estado: State<Estado>) -> Res<Option<Registro>> {
    db::registro_rodando(&estado.db.lock().unwrap()).map_err(erro)
}

#[tauri::command]
fn iniciar(app: AppHandle, estado: State<Estado>, atividade_id: i64) -> Res<()> {
    db::iniciar(&mut estado.db.lock().unwrap(), atividade_id, agora()).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn parar(app: AppHandle, estado: State<Estado>) -> Res<()> {
    db::parar(&estado.db.lock().unwrap(), agora()).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn listar_registros(estado: State<Estado>, de: i64, ate: i64) -> Res<Vec<Registro>> {
    db::listar_registros(&estado.db.lock().unwrap(), de, ate, agora()).map_err(erro)
}

#[tauri::command]
fn salvar_registro(app: AppHandle, estado: State<Estado>, registro: RegistroEntrada) -> Res<()> {
    if registro.inicio > agora() {
        return Err("O início não pode estar no futuro.".into());
    }
    match registro.fim {
        Some(fim) if fim <= registro.inicio => return Err("O fim precisa ser depois do início.".into()),
        None if registro.id.is_none() => return Err("Informe o horário de fim.".into()),
        _ => {}
    }
    db::salvar_registro(&estado.db.lock().unwrap(), &registro).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn excluir_registro(app: AppHandle, estado: State<Estado>, id: i64) -> Res<()> {
    db::excluir_registro(&estado.db.lock().unwrap(), id).map_err(erro)?;
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn obter_config(estado: State<Estado>) -> Config {
    estado.config.lock().unwrap().clone()
}

#[tauri::command]
fn salvar_config(app: AppHandle, estado: State<Estado>, config: Config) -> Res<()> {
    let anterior = estado.config.lock().unwrap().clone();
    if config.atalho != anterior.atalho {
        let gs = app.global_shortcut();
        gs.unregister_all().map_err(erro)?;
        if let Err(e) = gs.register(config.atalho.as_str()) {
            let _ = gs.register(anterior.atalho.as_str());
            return Err(format!("Atalho inválido: {e}"));
        }
    }
    if config.iniciar_com_windows != anterior.iniciar_com_windows && !cfg!(debug_assertions) {
        let al = app.autolaunch();
        let r = if config.iniciar_com_windows { al.enable() } else { al.disable() };
        r.map_err(erro)?;
    }
    db::salvar_config(&estado.db.lock().unwrap(), &config).map_err(erro)?;
    *estado.config.lock().unwrap() = config;
    atualizar_tray(&app);
    Ok(())
}

#[tauri::command]
fn ociosidade_pendente(estado: State<Estado>) -> Option<Ociosidade> {
    estado.ocio.lock().unwrap().pendente.clone()
}

/// acao: "manter" | "descartar" | "atribuir"
#[tauri::command]
fn resolver_ociosidade(app: AppHandle, estado: State<Estado>, acao: String, atividade_id: Option<i64>) -> Res<()> {
    let Some(p) = estado.ocio.lock().unwrap().pendente.take() else {
        return Ok(());
    };
    let _ = app.emit("ociosidade", None::<Ociosidade>);
    if acao == "manter" {
        return Ok(());
    }
    let outra = if acao == "atribuir" {
        Some(atividade_id.ok_or("Escolha a atividade.")?)
    } else {
        None
    };
    {
        let mut conn = estado.db.lock().unwrap();
        let rodando = db::registro_rodando(&conn).map_err(erro)?;
        match rodando {
            Some(r) if r.id == p.registro_id => {
                db::retirar_ociosidade(&mut conn, &r, p.inicio, p.fim, outra).map_err(erro)?
            }
            _ => return Err("O timer mudou desde a ausência.".into()),
        }
    }
    timer_mudou(&app);
    Ok(())
}

#[tauri::command]
fn fechar_seletor(app: AppHandle) {
    if let Some(w) = app.get_webview_window("seletor") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn mostrar_seletor(app: AppHandle) {
    abrir_seletor(&app);
}

// ---------- pasta de dados ----------

const BANCO: &str = "8pace.db";

/// Arquivo fixo em %APPDATA% que guarda qual pasta de dados usar.
fn arquivo_local(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("local.json"))
}

fn pasta_dados_configurada(app: &AppHandle) -> tauri::Result<PathBuf> {
    let padrao = app.path().app_data_dir()?;
    let escolhida = std::fs::read_to_string(arquivo_local(app)?)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["pasta_dados"].as_str().map(PathBuf::from));
    // pasta inacessível (ex.: unidade removida) cai no padrão
    Ok(match escolhida {
        Some(p) if std::fs::create_dir_all(&p).is_ok() => p,
        _ => padrao,
    })
}

fn copiar_pasta(origem: &Path, destino: &Path) -> std::io::Result<()> {
    if !origem.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(destino)?;
    for item in std::fs::read_dir(origem)? {
        let item = item?;
        let alvo = destino.join(item.file_name());
        if item.file_type()?.is_dir() {
            copiar_pasta(&item.path(), &alvo)?;
        } else {
            std::fs::copy(item.path(), alvo)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn pasta_dados(estado: State<Estado>) -> String {
    estado.pasta.display().to_string()
}

#[tauri::command]
fn pasta_tem_banco(caminho: String) -> bool {
    Path::new(&caminho).join(BANCO).exists()
}

/// Passa a usar `caminho` como pasta de dados. Com `copiar`, leva o banco e os anexos
/// atuais para lá (os originais ficam onde estão). Reinicia o app.
#[tauri::command]
fn mudar_pasta_dados(app: AppHandle, estado: State<Estado>, caminho: String, copiar: bool) -> Res<()> {
    let nova = PathBuf::from(&caminho);
    if !nova.is_dir() {
        return Err("Pasta não encontrada.".into());
    }
    if nova == estado.pasta {
        return Ok(());
    }
    if copiar {
        if nova.join(BANCO).exists() {
            return Err("A pasta já tem um banco do 8Pace.".into());
        }
        let conn = estado.db.lock().unwrap();
        conn.execute("VACUUM INTO ?1", [nova.join(BANCO).display().to_string()])
            .map_err(erro)?;
        copiar_pasta(&estado.pasta.join("anexos"), &nova.join("anexos")).map_err(erro)?;
    }
    let local = serde_json::json!({ "pasta_dados": caminho });
    std::fs::write(arquivo_local(&app).map_err(erro)?, local.to_string()).map_err(erro)?;
    app.restart();
}

#[tauri::command]
fn abrir_pasta_dados(app: AppHandle, estado: State<Estado>) -> Res<()> {
    app.opener()
        .open_path(estado.pasta.display().to_string(), None::<&str>)
        .map_err(erro)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            mostrar_janela(app, "main");
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _atalho, ev| {
                    if ev.state() == ShortcutState::Pressed {
                        abrir_seletor(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            std::fs::create_dir_all(app.path().app_data_dir()?)?;
            let pasta = pasta_dados_configurada(app.handle())?;
            std::fs::create_dir_all(pasta.join("anexos"))?;
            // miniaturas dos anexos via protocolo asset
            let _ = app.asset_protocol_scope().allow_directory(pasta.join("anexos"), true);
            let conn = db::abrir(&pasta.join(BANCO))?;
            let config = db::obter_config(&conn)?;
            let pausado = pausar_apos_desligamento(&conn)?;

            if let Err(e) = app.global_shortcut().register(config.atalho.as_str()) {
                eprintln!("não foi possível registrar o atalho {}: {e}", config.atalho);
            }
            if !cfg!(debug_assertions) {
                let al = app.autolaunch();
                let _ = if config.iniciar_com_windows { al.enable() } else { al.disable() };
            }

            app.manage(Estado {
                pasta,
                db: Mutex::new(conn),
                config: Mutex::new(config),
                ocio: Mutex::new(EstadoOcio::default()),
            });
            criar_tray(app.handle())?;
            atualizar_tray(app.handle());

            if let Some((atividade, quando)) = pausado {
                let hora = Local.timestamp_opt(quando, 0).unwrap().format("%d/%m %H:%M");
                let _ = app
                    .notification()
                    .builder()
                    .title("8Pace")
                    .body(format!("Timer de \"{atividade}\" pausado às {hora}, quando o 8Pace/PC foi desligado."))
                    .show();
            }

            if !std::env::args().any(|a| a == "--minimized") {
                mostrar_janela(app.handle(), "main");
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut contador = 0u64;
                loop {
                    std::thread::sleep(Duration::from_secs(5));
                    contador += 1;
                    ciclo(&handle, contador);
                }
            });
            Ok(())
        })
        .on_window_event(|janela, ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                api.prevent_close();
                let _ = janela.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            listar_projetos,
            criar_projeto,
            editar_projeto,
            excluir_projeto,
            finalizar_projeto,
            criar_atividade,
            renomear_atividade,
            listar_comentarios,
            salvar_comentario,
            excluir_comentario,
            abrir_link,
            gravar_pdf,
            dias_especiais,
            listar_folgas,
            criar_folga,
            excluir_folga,
            listar_listas,
            criar_lista,
            renomear_lista,
            excluir_lista,
            listar_tarefas,
            criar_tarefa,
            editar_tarefa,
            marcar_tarefa,
            mover_tarefa,
            excluir_tarefa,
            excluir_atividade,
            definir_status,
            definir_tipo,
            listar_tipos,
            criar_tipo,
            editar_tipo,
            excluir_tipo,
            total_atividade,
            listar_anexos,
            adicionar_anexos,
            colar_anexo,
            abrir_anexo,
            excluir_anexo,
            timer_atual,
            iniciar,
            parar,
            listar_registros,
            salvar_registro,
            excluir_registro,
            obter_config,
            salvar_config,
            ociosidade_pendente,
            resolver_ociosidade,
            fechar_seletor,
            mostrar_seletor,
            pasta_dados,
            pasta_tem_banco,
            mudar_pasta_dados,
            abrir_pasta_dados,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o 8Pace");
}
