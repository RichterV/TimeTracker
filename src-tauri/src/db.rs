use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

type Res<T> = rusqlite::Result<T>;

const MIGRACOES: &[&str] = &[r#"
    CREATE TABLE projeto (
        id INTEGER PRIMARY KEY,
        nome TEXT NOT NULL,
        cor TEXT NOT NULL DEFAULT '#4f8cff',
        arquivado INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE atividade (
        id INTEGER PRIMARY KEY,
        projeto_id INTEGER NOT NULL REFERENCES projeto(id) ON DELETE CASCADE,
        nome TEXT NOT NULL,
        arquivado INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE registro (
        id INTEGER PRIMARY KEY,
        atividade_id INTEGER NOT NULL REFERENCES atividade(id) ON DELETE CASCADE,
        inicio INTEGER NOT NULL,
        fim INTEGER,
        nota TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX idx_registro_inicio ON registro(inicio);
    -- garante no máximo um timer rodando
    CREATE UNIQUE INDEX idx_registro_rodando ON registro((fim IS NULL)) WHERE fim IS NULL;
    CREATE TABLE config (
        chave TEXT PRIMARY KEY,
        valor TEXT NOT NULL
    );
"#, r#"
    -- status substitui o "arquivado" das atividades (a coluna antiga fica sem uso)
    ALTER TABLE atividade ADD COLUMN status TEXT NOT NULL DEFAULT 'nao_iniciada';
    ALTER TABLE atividade ADD COLUMN descricao TEXT NOT NULL DEFAULT '';
    UPDATE atividade SET status = 'em_andamento'
        WHERE EXISTS (SELECT 1 FROM registro r WHERE r.atividade_id = atividade.id);
    UPDATE atividade SET status = 'finalizada' WHERE arquivado = 1;
    CREATE TABLE anexo (
        id INTEGER PRIMARY KEY,
        atividade_id INTEGER NOT NULL REFERENCES atividade(id) ON DELETE CASCADE,
        nome TEXT NOT NULL,
        arquivo TEXT NOT NULL DEFAULT '',
        tipo TEXT NOT NULL,
        tamanho INTEGER NOT NULL DEFAULT 0,
        criado INTEGER NOT NULL
    );
    CREATE INDEX idx_anexo_atividade ON anexo(atividade_id);
"#, r#"
    CREATE TABLE tipo (
        id INTEGER PRIMARY KEY,
        nome TEXT NOT NULL,
        icone TEXT NOT NULL,
        cor TEXT NOT NULL,
        ordem INTEGER NOT NULL DEFAULT 0
    );
    INSERT INTO tipo (nome, icone, cor, ordem) VALUES
        ('Bug fix', 'bug', '#d1453b', 1),
        ('Feature', 'sparkles', '#3b6fe0', 2),
        ('Melhoria', 'wrench', '#e0912f', 3),
        ('Análise de dados', 'chart-column', '#1fa2a8', 4),
        ('Pesquisa', 'search', '#8e5bd6', 5),
        ('Documentação', 'file-text', '#6b7380', 6),
        ('Reunião', 'users', '#c94f8a', 7),
        ('Revisão', 'eye', '#5b8cf5', 8),
        ('Suporte', 'headphones', '#2f9e5b', 9),
        ('Deploy/Infra', 'rocket', '#e5675e', 10),
        ('Estudo', 'book-open', '#9a7b3c', 11),
        ('Administrativo', 'clipboard-list', '#7a8496', 12);
    ALTER TABLE atividade ADD COLUMN tipo_id INTEGER REFERENCES tipo(id);
    UPDATE atividade SET tipo_id = (SELECT id FROM tipo WHERE nome = 'Feature');
    -- finalizado tem o mesmo efeito de arquivado, mas exibe a tag "Finalizado"
    ALTER TABLE projeto ADD COLUMN finalizado INTEGER NOT NULL DEFAULT 0;
"#, r#"
    -- linha do tempo de comentários; anexos citados no texto ficam ligados ao comentário
    CREATE TABLE comentario (
        id INTEGER PRIMARY KEY,
        atividade_id INTEGER NOT NULL REFERENCES atividade(id) ON DELETE CASCADE,
        texto TEXT NOT NULL,
        criado INTEGER NOT NULL,
        editado INTEGER
    );
    CREATE INDEX idx_comentario_atividade ON comentario(atividade_id);
    ALTER TABLE anexo ADD COLUMN comentario_id INTEGER REFERENCES comentario(id) ON DELETE SET NULL;
    -- a descrição antiga vira o primeiro comentário
    INSERT INTO comentario (atividade_id, texto, criado)
        SELECT id, descricao, CAST(strftime('%s', 'now') AS INTEGER) FROM atividade WHERE TRIM(descricao) <> '';
    UPDATE atividade SET descricao = '';
"#, r#"
    -- listas de tarefas (todo) com itens aninhados
    CREATE TABLE lista (
        id INTEGER PRIMARY KEY,
        nome TEXT NOT NULL,
        ordem INTEGER NOT NULL DEFAULT 0
    );
    INSERT INTO lista (nome, ordem) VALUES ('Geral', 1);
    CREATE TABLE tarefa (
        id INTEGER PRIMARY KEY,
        lista_id INTEGER NOT NULL REFERENCES lista(id) ON DELETE CASCADE,
        pai_id INTEGER REFERENCES tarefa(id) ON DELETE CASCADE,
        texto TEXT NOT NULL DEFAULT '',
        feito INTEGER NOT NULL DEFAULT 0,
        ordem INTEGER NOT NULL DEFAULT 0,
        criado INTEGER NOT NULL
    );
    CREATE INDEX idx_tarefa_lista ON tarefa(lista_id, pai_id, ordem);
"#, r#"
    -- folgas e férias (datas AAAA-MM-DD, inclusivas)
    CREATE TABLE folga (
        id INTEGER PRIMARY KEY,
        inicio TEXT NOT NULL,
        fim TEXT NOT NULL,
        descricao TEXT NOT NULL DEFAULT ''
    );
"#];

pub const STATUS: &[&str] = &["nao_iniciada", "em_andamento", "concluida", "finalizada"];

pub fn abrir(caminho: &Path) -> Res<Connection> {
    let conn = Connection::open(caminho)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let versao: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRACOES.iter().enumerate().skip(versao as usize) {
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", i as i64 + 1)?;
    }
    Ok(conn)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Intervalo {
    pub inicio: String,
    pub fim: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub ociosidade_min: u32,
    pub meta_diaria_min: u32,
    /// 1 = segunda ... 7 = domingo
    pub dias_uteis: Vec<u32>,
    pub expediente: Vec<Intervalo>,
    pub lembrete_ativo: bool,
    pub lembrete_intervalo_min: u32,
    pub atalho: String,
    pub iniciar_com_windows: bool,
    /// pontos facultativos considerados: "carnaval", "cinzas", "corpus"
    pub facultativos: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ociosidade_min: 10,
            meta_diaria_min: 480,
            dias_uteis: vec![1, 2, 3, 4, 5],
            expediente: vec![
                Intervalo { inicio: "08:00".into(), fim: "12:00".into() },
                Intervalo { inicio: "13:00".into(), fim: "17:00".into() },
            ],
            lembrete_ativo: true,
            lembrete_intervalo_min: 15,
            atalho: "Ctrl+Alt+Space".into(),
            iniciar_com_windows: true,
            facultativos: vec!["carnaval".into(), "cinzas".into(), "corpus".into()],
        }
    }
}

pub fn obter_config(conn: &Connection) -> Res<Config> {
    let valor: Option<String> = conn
        .query_row("SELECT valor FROM config WHERE chave = 'config'", [], |r| r.get(0))
        .optional()?;
    Ok(valor
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default())
}

pub fn salvar_config(conn: &Connection, config: &Config) -> Res<()> {
    let valor = serde_json::to_string(config).unwrap();
    conn.execute(
        "INSERT INTO config (chave, valor) VALUES ('config', ?1)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor",
        params![valor],
    )?;
    Ok(())
}

#[derive(Serialize)]
pub struct Atividade {
    pub id: i64,
    pub projeto_id: i64,
    pub nome: String,
    pub status: String,
    pub descricao: String,
    pub tipo_id: Option<i64>,
}

#[derive(Serialize)]
pub struct Projeto {
    pub id: i64,
    pub nome: String,
    pub cor: String,
    pub arquivado: bool,
    pub finalizado: bool,
    pub atividades: Vec<Atividade>,
}

pub fn listar_projetos(conn: &Connection) -> Res<Vec<Projeto>> {
    let mut stmt = conn.prepare(
        "SELECT id, nome, cor, arquivado, finalizado FROM projeto ORDER BY nome COLLATE NOCASE",
    )?;
    let mut projetos = stmt
        .query_map([], |r| {
            Ok(Projeto {
                id: r.get(0)?,
                nome: r.get(1)?,
                cor: r.get(2)?,
                arquivado: r.get(3)?,
                finalizado: r.get(4)?,
                atividades: Vec::new(),
            })
        })?
        .collect::<Res<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, projeto_id, nome, status, descricao, tipo_id FROM atividade ORDER BY nome COLLATE NOCASE",
    )?;
    let atividades = stmt.query_map([], |r| {
        Ok(Atividade {
            id: r.get(0)?,
            projeto_id: r.get(1)?,
            nome: r.get(2)?,
            status: r.get(3)?,
            descricao: r.get(4)?,
            tipo_id: r.get(5)?,
        })
    })?;
    for a in atividades {
        let a = a?;
        if let Some(p) = projetos.iter_mut().find(|p| p.id == a.projeto_id) {
            p.atividades.push(a);
        }
    }
    Ok(projetos)
}

pub fn criar_projeto(conn: &Connection, nome: &str, cor: &str) -> Res<i64> {
    conn.execute("INSERT INTO projeto (nome, cor) VALUES (?1, ?2)", params![nome, cor])?;
    Ok(conn.last_insert_rowid())
}

pub fn editar_projeto(conn: &Connection, id: i64, nome: &str, cor: &str, arquivado: bool) -> Res<()> {
    // arquivar tira o "finalizado": o projeto fica em um só desses estados
    conn.execute(
        "UPDATE projeto SET nome = ?2, cor = ?3, arquivado = ?4,
                finalizado = CASE WHEN ?4 THEN 0 ELSE finalizado END
         WHERE id = ?1",
        params![id, nome, cor, arquivado],
    )?;
    Ok(())
}

pub fn finalizar_projeto(conn: &Connection, id: i64, finalizado: bool) -> Res<()> {
    conn.execute(
        "UPDATE projeto SET finalizado = ?2, arquivado = 0 WHERE id = ?1",
        params![id, finalizado],
    )?;
    Ok(())
}

pub fn excluir_projeto(conn: &Connection, id: i64) -> Res<bool> {
    let usados: i64 = conn.query_row(
        "SELECT COUNT(*) FROM registro r JOIN atividade a ON a.id = r.atividade_id WHERE a.projeto_id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    if usados > 0 {
        return Ok(false);
    }
    conn.execute("DELETE FROM projeto WHERE id = ?1", params![id])?;
    Ok(true)
}

pub fn criar_atividade(conn: &Connection, projeto_id: i64, nome: &str, tipo_id: i64) -> Res<i64> {
    conn.execute(
        "INSERT INTO atividade (projeto_id, nome, tipo_id) VALUES (?1, ?2, ?3)",
        params![projeto_id, nome, tipo_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn definir_tipo(conn: &Connection, id: i64, tipo_id: i64) -> Res<()> {
    conn.execute("UPDATE atividade SET tipo_id = ?2 WHERE id = ?1", params![id, tipo_id])?;
    Ok(())
}

#[derive(Serialize)]
pub struct Tipo {
    pub id: i64,
    pub nome: String,
    pub icone: String,
    pub cor: String,
    pub ordem: i64,
}

pub fn listar_tipos(conn: &Connection) -> Res<Vec<Tipo>> {
    let mut stmt = conn.prepare("SELECT id, nome, icone, cor, ordem FROM tipo ORDER BY ordem, id")?;
    stmt.query_map([], |r| {
        Ok(Tipo {
            id: r.get(0)?,
            nome: r.get(1)?,
            icone: r.get(2)?,
            cor: r.get(3)?,
            ordem: r.get(4)?,
        })
    })?
    .collect()
}

pub fn criar_tipo(conn: &Connection, nome: &str, icone: &str, cor: &str) -> Res<i64> {
    conn.execute(
        "INSERT INTO tipo (nome, icone, cor, ordem)
         VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(ordem), 0) + 1 FROM tipo))",
        params![nome, icone, cor],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn editar_tipo(conn: &Connection, id: i64, nome: &str, icone: &str, cor: &str) -> Res<()> {
    conn.execute(
        "UPDATE tipo SET nome = ?2, icone = ?3, cor = ?4 WHERE id = ?1",
        params![id, nome, icone, cor],
    )?;
    Ok(())
}

/// Exclui o tipo se nenhuma atividade usa. Devolve quantas usam (0 = excluído).
pub fn excluir_tipo(conn: &Connection, id: i64) -> Res<i64> {
    let usados: i64 = conn.query_row(
        "SELECT COUNT(*) FROM atividade WHERE tipo_id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    if usados == 0 {
        conn.execute("DELETE FROM tipo WHERE id = ?1", params![id])?;
    }
    Ok(usados)
}

pub fn renomear_atividade(conn: &Connection, id: i64, nome: &str) -> Res<()> {
    conn.execute("UPDATE atividade SET nome = ?2 WHERE id = ?1", params![id, nome])?;
    Ok(())
}

#[derive(Serialize)]
pub struct Comentario {
    pub id: i64,
    pub atividade_id: i64,
    pub texto: String,
    pub criado: i64,
    pub editado: Option<i64>,
}

const SELECT_COMENTARIO: &str = "SELECT id, atividade_id, texto, criado, editado FROM comentario";

fn ler_comentario(r: &rusqlite::Row) -> Res<Comentario> {
    Ok(Comentario {
        id: r.get(0)?,
        atividade_id: r.get(1)?,
        texto: r.get(2)?,
        criado: r.get(3)?,
        editado: r.get(4)?,
    })
}

pub fn listar_comentarios(conn: &Connection, atividade_id: i64) -> Res<Vec<Comentario>> {
    let mut stmt = conn.prepare(&format!("{SELECT_COMENTARIO} WHERE atividade_id = ?1 ORDER BY criado, id"))?;
    stmt.query_map(params![atividade_id], ler_comentario)?.collect()
}

/// IDs citados no texto como {{anexo:ID}} ou {{anexo:ID|nome}}.
pub fn anexos_citados(texto: &str) -> Vec<i64> {
    texto
        .split("{{anexo:")
        .skip(1)
        .filter_map(|resto| {
            let digitos: String = resto.chars().take_while(|c| c.is_ascii_digit()).collect();
            digitos.parse().ok()
        })
        .collect()
}

/// Cria ou atualiza o comentário e liga a ele os anexos citados no texto.
pub fn salvar_comentario(
    conn: &mut Connection,
    atividade_id: i64,
    id: Option<i64>,
    texto: &str,
    agora: i64,
) -> Res<Comentario> {
    let tx = conn.transaction()?;
    let id = match id {
        Some(id) => {
            tx.execute(
                "UPDATE comentario SET texto = ?2, editado = ?3 WHERE id = ?1",
                params![id, texto, agora],
            )?;
            id
        }
        None => {
            tx.execute(
                "INSERT INTO comentario (atividade_id, texto, criado) VALUES (?1, ?2, ?3)",
                params![atividade_id, texto, agora],
            )?;
            tx.last_insert_rowid()
        }
    };
    let citados = anexos_citados(texto);
    // anexos que saíram do texto voltam a ser soltos (ficam só no índice)
    tx.execute("UPDATE anexo SET comentario_id = NULL WHERE comentario_id = ?1", params![id])?;
    for anexo in citados {
        tx.execute(
            "UPDATE anexo SET comentario_id = ?1 WHERE id = ?2 AND atividade_id = ?3",
            params![id, anexo, atividade_id],
        )?;
    }
    let comentario = tx.query_row(&format!("{SELECT_COMENTARIO} WHERE id = ?1"), params![id], ler_comentario)?;
    tx.commit()?;
    Ok(comentario)
}

/// Exclui o comentário e devolve os anexos dele (para apagar os arquivos).
pub fn excluir_comentario(conn: &mut Connection, id: i64) -> Res<Vec<Anexo>> {
    let tx = conn.transaction()?;
    let anexos = {
        let mut stmt = tx.prepare(&format!("{SELECT_ANEXO} WHERE comentario_id = ?1"))?;
        stmt.query_map(params![id], ler_anexo)?.collect::<Res<Vec<_>>>()?
    };
    tx.execute("DELETE FROM anexo WHERE comentario_id = ?1", params![id])?;
    tx.execute("DELETE FROM comentario WHERE id = ?1", params![id])?;
    tx.commit()?;
    Ok(anexos)
}

pub fn definir_status(conn: &Connection, id: i64, status: &str) -> Res<()> {
    conn.execute("UPDATE atividade SET status = ?2 WHERE id = ?1", params![id, status])?;
    Ok(())
}

pub fn atividades_do_projeto(conn: &Connection, projeto_id: i64) -> Res<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM atividade WHERE projeto_id = ?1")?;
    stmt.query_map(params![projeto_id], |r| r.get(0))?.collect()
}

/// Segundos totais registrados na atividade.
pub fn total_atividade(conn: &Connection, id: i64, agora: i64) -> Res<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(fim, ?2) - inicio), 0) FROM registro WHERE atividade_id = ?1",
        params![id, agora],
        |r| r.get(0),
    )
}

pub fn excluir_atividade(conn: &Connection, id: i64) -> Res<bool> {
    let usados: i64 = conn.query_row(
        "SELECT COUNT(*) FROM registro WHERE atividade_id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    if usados > 0 {
        return Ok(false);
    }
    conn.execute("DELETE FROM atividade WHERE id = ?1", params![id])?;
    Ok(true)
}

#[derive(Serialize, Clone)]
pub struct Registro {
    pub id: i64,
    pub atividade_id: i64,
    pub atividade: String,
    pub projeto: String,
    pub cor: String,
    pub inicio: i64,
    pub fim: Option<i64>,
    pub nota: String,
}

const SELECT_REGISTRO: &str = "
    SELECT r.id, r.atividade_id, a.nome, p.nome, p.cor, r.inicio, r.fim, r.nota
    FROM registro r
    JOIN atividade a ON a.id = r.atividade_id
    JOIN projeto p ON p.id = a.projeto_id";

fn ler_registro(r: &rusqlite::Row) -> Res<Registro> {
    Ok(Registro {
        id: r.get(0)?,
        atividade_id: r.get(1)?,
        atividade: r.get(2)?,
        projeto: r.get(3)?,
        cor: r.get(4)?,
        inicio: r.get(5)?,
        fim: r.get(6)?,
        nota: r.get(7)?,
    })
}

pub fn registro_rodando(conn: &Connection) -> Res<Option<Registro>> {
    conn.query_row(&format!("{SELECT_REGISTRO} WHERE r.fim IS NULL"), [], ler_registro)
        .optional()
}

/// Registros que tocam o intervalo [de, ate).
pub fn listar_registros(conn: &Connection, de: i64, ate: i64, agora: i64) -> Res<Vec<Registro>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT_REGISTRO} WHERE r.inicio < ?2 AND COALESCE(r.fim, ?3) > ?1 ORDER BY r.inicio"
    ))?;
    stmt.query_map(params![de, ate, agora], ler_registro)?.collect()
}

/// Soma dos segundos registrados dentro de [de, ate).
pub fn total_periodo(conn: &Connection, de: i64, ate: i64, agora: i64) -> Res<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(MIN(COALESCE(fim, ?3), ?2) - MAX(inicio, ?1)), 0)
         FROM registro WHERE inicio < ?2 AND COALESCE(fim, ?3) > ?1",
        params![de, ate, agora],
        |r| r.get(0),
    )
}

pub fn iniciar(conn: &mut Connection, atividade_id: i64, agora: i64) -> Res<()> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE registro SET fim = ?1 WHERE fim IS NULL", params![agora])?;
    tx.execute(
        "INSERT INTO registro (atividade_id, inicio) VALUES (?1, ?2)",
        params![atividade_id, agora],
    )?;
    // iniciar o timer coloca (ou devolve) a atividade em andamento
    tx.execute(
        "UPDATE atividade SET status = 'em_andamento' WHERE id = ?1",
        params![atividade_id],
    )?;
    tx.commit()
}

/// Marca que o app estava vivo em `agora` (usado para fechar o timer se o PC desligar).
pub fn registrar_batimento(conn: &Connection, agora: i64) -> Res<()> {
    conn.execute(
        "INSERT INTO config (chave, valor) VALUES ('batimento', ?1)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor",
        params![agora.to_string()],
    )?;
    Ok(())
}

pub fn ultimo_batimento(conn: &Connection) -> Res<Option<i64>> {
    let valor: Option<String> = conn
        .query_row("SELECT valor FROM config WHERE chave = 'batimento'", [], |r| r.get(0))
        .optional()?;
    Ok(valor.and_then(|v| v.parse().ok()))
}

pub fn parar(conn: &Connection, agora: i64) -> Res<()> {
    conn.execute("UPDATE registro SET fim = ?1 WHERE fim IS NULL", params![agora])?;
    Ok(())
}

#[derive(Deserialize)]
pub struct RegistroEntrada {
    pub id: Option<i64>,
    pub atividade_id: i64,
    pub inicio: i64,
    pub fim: Option<i64>,
    pub nota: String,
}

pub fn salvar_registro(conn: &Connection, r: &RegistroEntrada) -> Res<()> {
    match r.id {
        Some(id) => conn.execute(
            "UPDATE registro SET atividade_id = ?2, inicio = ?3, fim = ?4, nota = ?5 WHERE id = ?1",
            params![id, r.atividade_id, r.inicio, r.fim, r.nota],
        )?,
        None => conn.execute(
            "INSERT INTO registro (atividade_id, inicio, fim, nota) VALUES (?1, ?2, ?3, ?4)",
            params![r.atividade_id, r.inicio, r.fim, r.nota],
        )?,
    };
    Ok(())
}

pub fn excluir_registro(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM registro WHERE id = ?1", params![id])?;
    Ok(())
}

#[derive(Serialize)]
pub struct Anexo {
    pub id: i64,
    pub atividade_id: i64,
    pub nome: String,
    /// caminho relativo à pasta de anexos
    pub arquivo: String,
    pub tipo: String,
    pub tamanho: i64,
    pub criado: i64,
    /// caminho absoluto, preenchido pela camada de comandos
    pub caminho: String,
    pub comentario_id: Option<i64>,
}

const SELECT_ANEXO: &str =
    "SELECT id, atividade_id, nome, arquivo, tipo, tamanho, criado, comentario_id FROM anexo";

fn ler_anexo(r: &rusqlite::Row) -> Res<Anexo> {
    Ok(Anexo {
        id: r.get(0)?,
        atividade_id: r.get(1)?,
        nome: r.get(2)?,
        arquivo: r.get(3)?,
        tipo: r.get(4)?,
        tamanho: r.get(5)?,
        criado: r.get(6)?,
        caminho: String::new(),
        comentario_id: r.get(7)?,
    })
}

pub fn inserir_anexo(conn: &Connection, atividade_id: i64, nome: &str, tipo: &str, criado: i64) -> Res<i64> {
    conn.execute(
        "INSERT INTO anexo (atividade_id, nome, tipo, criado) VALUES (?1, ?2, ?3, ?4)",
        params![atividade_id, nome, tipo, criado],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn concluir_anexo(conn: &Connection, id: i64, arquivo: &str, tamanho: i64) -> Res<()> {
    conn.execute(
        "UPDATE anexo SET arquivo = ?2, tamanho = ?3 WHERE id = ?1",
        params![id, arquivo, tamanho],
    )?;
    Ok(())
}

pub fn obter_anexo(conn: &Connection, id: i64) -> Res<Option<Anexo>> {
    conn.query_row(&format!("{SELECT_ANEXO} WHERE id = ?1"), params![id], ler_anexo)
        .optional()
}

pub fn listar_anexos(conn: &Connection, atividade_id: i64) -> Res<Vec<Anexo>> {
    let mut stmt = conn.prepare(&format!("{SELECT_ANEXO} WHERE atividade_id = ?1 ORDER BY criado"))?;
    stmt.query_map(params![atividade_id], ler_anexo)?.collect()
}

pub fn excluir_anexo(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM anexo WHERE id = ?1", params![id])?;
    Ok(())
}

/// Retira o período ocioso [ini, fim) do registro rodando. O timer segue na mesma
/// atividade a partir de `fim`. Se `outra` vier preenchida, o período vai para ela.
pub fn retirar_ociosidade(
    conn: &mut Connection,
    rodando: &Registro,
    ini: i64,
    fim: i64,
    outra: Option<i64>,
) -> Res<()> {
    let ini = ini.max(rodando.inicio);
    if ini >= fim {
        return Ok(());
    }
    let tx = conn.transaction()?;
    if ini == rodando.inicio {
        tx.execute("UPDATE registro SET inicio = ?2 WHERE id = ?1", params![rodando.id, fim])?;
    } else {
        tx.execute("UPDATE registro SET fim = ?2 WHERE id = ?1", params![rodando.id, ini])?;
        tx.execute(
            "INSERT INTO registro (atividade_id, inicio) VALUES (?1, ?2)",
            params![rodando.atividade_id, fim],
        )?;
    }
    if let Some(atividade_id) = outra {
        tx.execute(
            "INSERT INTO registro (atividade_id, inicio, fim) VALUES (?1, ?2, ?3)",
            params![atividade_id, ini, fim],
        )?;
    }
    tx.commit()
}
