//! Listas de tarefas com itens aninhados (todo list).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

type Res<T> = rusqlite::Result<T>;

#[derive(Serialize)]
pub struct Lista {
    pub id: i64,
    pub nome: String,
    pub ordem: i64,
}

#[derive(Serialize)]
pub struct Tarefa {
    pub id: i64,
    pub lista_id: i64,
    pub pai_id: Option<i64>,
    pub texto: String,
    pub feito: bool,
    pub ordem: i64,
}

pub fn listar_listas(conn: &Connection) -> Res<Vec<Lista>> {
    let mut stmt = conn.prepare("SELECT id, nome, ordem FROM lista ORDER BY ordem, id")?;
    stmt.query_map([], |r| Ok(Lista { id: r.get(0)?, nome: r.get(1)?, ordem: r.get(2)? }))?
        .collect()
}

pub fn criar_lista(conn: &Connection, nome: &str) -> Res<i64> {
    conn.execute(
        "INSERT INTO lista (nome, ordem) VALUES (?1, (SELECT COALESCE(MAX(ordem), 0) + 1 FROM lista))",
        params![nome],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn renomear_lista(conn: &Connection, id: i64, nome: &str) -> Res<()> {
    conn.execute("UPDATE lista SET nome = ?2 WHERE id = ?1", params![id, nome])?;
    Ok(())
}

pub fn excluir_lista(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM lista WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn listar_tarefas(conn: &Connection, lista_id: i64) -> Res<Vec<Tarefa>> {
    let mut stmt = conn.prepare(
        "SELECT id, lista_id, pai_id, texto, feito, ordem FROM tarefa WHERE lista_id = ?1 ORDER BY ordem, id",
    )?;
    stmt.query_map(params![lista_id], |r| {
        Ok(Tarefa {
            id: r.get(0)?,
            lista_id: r.get(1)?,
            pai_id: r.get(2)?,
            texto: r.get(3)?,
            feito: r.get(4)?,
            ordem: r.get(5)?,
        })
    })?
    .collect()
}

fn lista_e_pai(conn: &Connection, id: i64) -> Res<(i64, Option<i64>)> {
    conn.query_row("SELECT lista_id, pai_id FROM tarefa WHERE id = ?1", params![id], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })
}

/// IDs dos irmãos (mesmo pai), na ordem atual.
fn irmaos(conn: &Connection, lista_id: i64, pai_id: Option<i64>) -> Res<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT id FROM tarefa WHERE lista_id = ?1 AND pai_id IS ?2 ORDER BY ordem, id")?;
    stmt.query_map(params![lista_id, pai_id], |r| r.get(0))?.collect()
}

fn renumerar(conn: &Connection, ids: &[i64]) -> Res<()> {
    for (i, id) in ids.iter().enumerate() {
        conn.execute("UPDATE tarefa SET ordem = ?2 WHERE id = ?1", params![id, i as i64])?;
    }
    Ok(())
}

/// Sobe a partir de `pai`: cada ancestral fica feito só se todos os filhos estiverem feitos.
fn recalcular_pais(conn: &Connection, mut pai: Option<i64>) -> Res<()> {
    while let Some(p) = pai {
        let (total, feitos): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(feito), 0) FROM tarefa WHERE pai_id = ?1",
            params![p],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if total > 0 {
            conn.execute("UPDATE tarefa SET feito = ?2 WHERE id = ?1", params![p, total == feitos])?;
        }
        pai = conn
            .query_row("SELECT pai_id FROM tarefa WHERE id = ?1", params![p], |r| r.get(0))
            .optional()?
            .flatten();
    }
    Ok(())
}

/// Cria um item logo depois de `apos` (entre os irmãos) ou no fim.
pub fn criar_tarefa(
    conn: &mut Connection,
    lista_id: i64,
    pai_id: Option<i64>,
    apos: Option<i64>,
    texto: &str,
    agora: i64,
) -> Res<i64> {
    let tx = conn.transaction()?;
    let mut ids = irmaos(&tx, lista_id, pai_id)?;
    tx.execute(
        "INSERT INTO tarefa (lista_id, pai_id, texto, criado) VALUES (?1, ?2, ?3, ?4)",
        params![lista_id, pai_id, texto, agora],
    )?;
    let id = tx.last_insert_rowid();
    let pos = apos
        .and_then(|a| ids.iter().position(|&x| x == a))
        .map(|p| p + 1)
        .unwrap_or(ids.len());
    ids.insert(pos, id);
    renumerar(&tx, &ids)?;
    recalcular_pais(&tx, pai_id)?;
    tx.commit()?;
    Ok(id)
}

pub fn editar_tarefa(conn: &Connection, id: i64, texto: &str) -> Res<()> {
    conn.execute("UPDATE tarefa SET texto = ?2 WHERE id = ?1", params![id, texto])?;
    Ok(())
}

/// Marca/desmarca o item e todos os descendentes, depois ajusta os ancestrais.
pub fn marcar_tarefa(conn: &mut Connection, id: i64, feito: bool) -> Res<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "WITH RECURSIVE d(id) AS (
             SELECT ?1 UNION ALL SELECT t.id FROM tarefa t JOIN d ON t.pai_id = d.id
         )
         UPDATE tarefa SET feito = ?2 WHERE id IN d",
        params![id, feito],
    )?;
    let (_, pai) = lista_e_pai(&tx, id)?;
    recalcular_pais(&tx, pai)?;
    tx.commit()
}

/// Move o item para `novo_pai`, na posição `posicao` entre os novos irmãos.
/// Devolve false se o destino for o próprio item ou um descendente dele.
pub fn mover_tarefa(conn: &mut Connection, id: i64, novo_pai: Option<i64>, posicao: usize) -> Res<bool> {
    let tx = conn.transaction()?;
    let mut atual = novo_pai;
    while let Some(p) = atual {
        if p == id {
            return Ok(false);
        }
        atual = lista_e_pai(&tx, p)?.1;
    }
    let (lista_id, pai_antigo) = lista_e_pai(&tx, id)?;

    let antigos: Vec<i64> = irmaos(&tx, lista_id, pai_antigo)?.into_iter().filter(|&x| x != id).collect();
    renumerar(&tx, &antigos)?;

    tx.execute("UPDATE tarefa SET pai_id = ?2 WHERE id = ?1", params![id, novo_pai])?;
    let mut novos: Vec<i64> = irmaos(&tx, lista_id, novo_pai)?.into_iter().filter(|&x| x != id).collect();
    novos.insert(posicao.min(novos.len()), id);
    renumerar(&tx, &novos)?;

    recalcular_pais(&tx, pai_antigo)?;
    recalcular_pais(&tx, novo_pai)?;
    tx.commit()?;
    Ok(true)
}

/// Exclui o item e os subitens.
pub fn excluir_tarefa(conn: &mut Connection, id: i64) -> Res<()> {
    let tx = conn.transaction()?;
    let (lista_id, pai) = lista_e_pai(&tx, id)?;
    tx.execute("DELETE FROM tarefa WHERE id = ?1", params![id])?;
    renumerar(&tx, &irmaos(&tx, lista_id, pai)?)?;
    recalcular_pais(&tx, pai)?;
    tx.commit()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn banco() -> (Connection, i64) {
        let conn = crate::db::abrir(std::path::Path::new(":memory:")).unwrap();
        let lista = listar_listas(&conn).unwrap()[0].id;
        (conn, lista)
    }

    fn feito(conn: &Connection, id: i64) -> bool {
        conn.query_row("SELECT feito FROM tarefa WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn marcar_propaga_para_filhos_e_pais() {
        let (mut conn, l) = banco();
        let pai = criar_tarefa(&mut conn, l, None, None, "pai", 0).unwrap();
        let a = criar_tarefa(&mut conn, l, Some(pai), None, "a", 0).unwrap();
        let b = criar_tarefa(&mut conn, l, Some(pai), None, "b", 0).unwrap();

        marcar_tarefa(&mut conn, a, true).unwrap();
        assert!(!feito(&conn, pai));
        marcar_tarefa(&mut conn, b, true).unwrap();
        assert!(feito(&conn, pai), "pai conclui quando todos os filhos concluem");

        marcar_tarefa(&mut conn, b, false).unwrap();
        assert!(!feito(&conn, pai), "desmarcar filho reabre o pai");

        marcar_tarefa(&mut conn, pai, true).unwrap();
        assert!(feito(&conn, a) && feito(&conn, b), "marcar pai marca filhos");
    }

    #[test]
    fn mover_reordena_aninha_e_impede_ciclo() {
        let (mut conn, l) = banco();
        let x = criar_tarefa(&mut conn, l, None, None, "x", 0).unwrap();
        let y = criar_tarefa(&mut conn, l, None, None, "y", 0).unwrap();
        let z = criar_tarefa(&mut conn, l, None, Some(x), "z", 0).unwrap();
        assert_eq!(irmaos(&conn, l, None).unwrap(), vec![x, z, y], "criar após x");

        assert!(mover_tarefa(&mut conn, y, Some(x), 1_000_000).unwrap());
        assert_eq!(irmaos(&conn, l, Some(x)).unwrap(), vec![y]);
        assert!(!mover_tarefa(&mut conn, x, Some(y), 0).unwrap(), "não entra no próprio descendente");

        assert!(mover_tarefa(&mut conn, z, None, 0).unwrap());
        assert_eq!(irmaos(&conn, l, None).unwrap(), vec![z, x]);

        excluir_tarefa(&mut conn, x).unwrap();
        assert_eq!(listar_tarefas(&conn, l).unwrap().len(), 1, "excluir leva os subitens");
    }
}
