//! Feriados nacionais do Brasil (calculados), pontos facultativos e folgas cadastradas.

use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::{params, Connection};
use serde::Serialize;

type Res<T> = rusqlite::Result<T>;

/// Dia com expediente diferente do normal.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DiaEspecial {
    /// AAAA-MM-DD
    pub data: String,
    pub nome: String,
    /// "feriado" | "folga" | "facultativo" | "meio" (meio período)
    pub tipo: String,
    /// Para meio período: horário em que o expediente começa ("14:00").
    pub a_partir: Option<String>,
}

impl DiaEspecial {
    fn novo(data: NaiveDate, nome: &str, tipo: &str, a_partir: Option<&str>) -> Self {
        Self {
            data: data.format("%Y-%m-%d").to_string(),
            nome: nome.into(),
            tipo: tipo.into(),
            a_partir: a_partir.map(Into::into),
        }
    }

    fn prioridade(&self) -> u8 {
        match self.tipo.as_str() {
            "feriado" => 0,
            "folga" => 1,
            "facultativo" => 2,
            _ => 3,
        }
    }
}

/// Domingo de Páscoa (algoritmo de Meeus/Jones/Butcher).
pub fn pascoa(ano: i32) -> NaiveDate {
    let a = ano % 19;
    let b = ano / 100;
    let c = ano % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let mes = (h + l - 7 * m + 114) / 31;
    let dia = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(ano, mes as u32, dia as u32).unwrap()
}

/// Feriados nacionais e os pontos facultativos escolhidos (`carnaval`, `cinzas`, `corpus`).
pub fn do_ano(ano: i32, facultativos: &[String]) -> Vec<DiaEspecial> {
    let d = |m: u32, dia: u32| NaiveDate::from_ymd_opt(ano, m, dia).unwrap();
    let mut v = vec![
        DiaEspecial::novo(d(1, 1), "Confraternização Universal", "feriado", None),
        DiaEspecial::novo(d(4, 21), "Tiradentes", "feriado", None),
        DiaEspecial::novo(d(5, 1), "Dia do Trabalho", "feriado", None),
        DiaEspecial::novo(d(9, 7), "Independência do Brasil", "feriado", None),
        DiaEspecial::novo(d(10, 12), "Nossa Senhora Aparecida", "feriado", None),
        DiaEspecial::novo(d(11, 2), "Finados", "feriado", None),
        DiaEspecial::novo(d(11, 15), "Proclamação da República", "feriado", None),
        DiaEspecial::novo(d(12, 25), "Natal", "feriado", None),
    ];
    if ano >= 2024 {
        v.push(DiaEspecial::novo(d(11, 20), "Consciência Negra", "feriado", None));
    }
    let p = pascoa(ano);
    v.push(DiaEspecial::novo(p - Duration::days(2), "Sexta-feira Santa", "feriado", None));
    let quer = |k: &str| facultativos.iter().any(|f| f == k);
    if quer("carnaval") {
        v.push(DiaEspecial::novo(p - Duration::days(48), "Carnaval", "facultativo", None));
        v.push(DiaEspecial::novo(p - Duration::days(47), "Carnaval", "facultativo", None));
    }
    if quer("cinzas") {
        v.push(DiaEspecial::novo(p - Duration::days(46), "Quarta de Cinzas (até 14h)", "meio", Some("14:00")));
    }
    if quer("corpus") {
        v.push(DiaEspecial::novo(p + Duration::days(60), "Corpus Christi", "facultativo", None));
    }
    v
}

#[derive(Serialize)]
pub struct Folga {
    pub id: i64,
    pub inicio: String,
    pub fim: String,
    pub descricao: String,
}

pub fn listar_folgas(conn: &Connection) -> Res<Vec<Folga>> {
    let mut stmt = conn.prepare("SELECT id, inicio, fim, descricao FROM folga ORDER BY inicio DESC")?;
    stmt.query_map([], |r| {
        Ok(Folga { id: r.get(0)?, inicio: r.get(1)?, fim: r.get(2)?, descricao: r.get(3)? })
    })?
    .collect()
}

pub fn criar_folga(conn: &Connection, inicio: &str, fim: &str, descricao: &str) -> Res<i64> {
    conn.execute(
        "INSERT INTO folga (inicio, fim, descricao) VALUES (?1, ?2, ?3)",
        params![inicio, fim, descricao],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn excluir_folga(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM folga WHERE id = ?1", params![id])?;
    Ok(())
}

/// Dias especiais em [de, ate] (inclusive), um por data, pelo de maior prioridade.
pub fn no_periodo(conn: &Connection, de: NaiveDate, ate: NaiveDate, facultativos: &[String]) -> Res<Vec<DiaEspecial>> {
    let mut todos: Vec<DiaEspecial> = (de.year()..=ate.year()).flat_map(|a| do_ano(a, facultativos)).collect();

    let mut stmt = conn.prepare("SELECT inicio, fim, descricao FROM folga WHERE inicio <= ?2 AND fim >= ?1")?;
    let folgas = stmt.query_map(
        params![de.format("%Y-%m-%d").to_string(), ate.format("%Y-%m-%d").to_string()],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
    )?;
    for f in folgas {
        let (ini, fim, desc) = f?;
        let (Ok(ini), Ok(fim)) = (NaiveDate::parse_from_str(&ini, "%Y-%m-%d"), NaiveDate::parse_from_str(&fim, "%Y-%m-%d")) else {
            continue;
        };
        let nome = if desc.trim().is_empty() { "Folga" } else { desc.trim() };
        let mut d = ini.max(de);
        while d <= fim.min(ate) {
            todos.push(DiaEspecial::novo(d, nome, "folga", None));
            d += Duration::days(1);
        }
    }

    let (de_s, ate_s) = (de.format("%Y-%m-%d").to_string(), ate.format("%Y-%m-%d").to_string());
    todos.retain(|d| d.data >= de_s && d.data <= ate_s);
    todos.sort_by(|a, b| a.data.cmp(&b.data).then(a.prioridade().cmp(&b.prioridade())));
    todos.dedup_by(|b, a| a.data == b.data);
    Ok(todos)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn data(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn pascoa_conhecida() {
        assert_eq!(pascoa(2024), data("2024-03-31"));
        assert_eq!(pascoa(2025), data("2025-04-20"));
        assert_eq!(pascoa(2026), data("2026-04-05"));
        assert_eq!(pascoa(2027), data("2027-03-28"));
    }

    #[test]
    fn moveis_de_2026() {
        let todos = vec!["carnaval".into(), "cinzas".into(), "corpus".into()];
        let v = do_ano(2026, &todos);
        let tem = |d: &str, tipo: &str| v.iter().any(|x| x.data == d && x.tipo == tipo);
        assert!(tem("2026-02-16", "facultativo") && tem("2026-02-17", "facultativo"), "carnaval");
        assert!(tem("2026-02-18", "meio"), "cinzas");
        assert!(tem("2026-04-03", "feriado"), "sexta-feira santa");
        assert!(tem("2026-06-04", "facultativo"), "corpus christi");
        assert!(tem("2026-11-20", "feriado"), "consciência negra");
        assert!(!do_ano(2026, &[]).iter().any(|x| x.tipo != "feriado"), "sem facultativos");
    }

    #[test]
    fn folga_e_prioridade() {
        let conn = crate::db::abrir(std::path::Path::new(":memory:")).unwrap();
        criar_folga(&conn, "2026-12-24", "2026-12-26", "Férias").unwrap();
        let v = no_periodo(&conn, data("2026-12-23"), data("2026-12-27"), &[]).unwrap();
        let datas: Vec<(&str, &str)> = v.iter().map(|d| (d.data.as_str(), d.tipo.as_str())).collect();
        assert_eq!(datas, vec![("2026-12-24", "folga"), ("2026-12-25", "feriado"), ("2026-12-26", "folga")]);
    }
}
