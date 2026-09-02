use chrono::{Datelike, NaiveDate, NaiveDateTime};
use directories::ProjectDirs;
use rusqlite::Connection;
use std::path::PathBuf;

pub const SALAS: [&str; 3] = ["General", "Estudio", "Computo"];

pub fn data_dir() -> PathBuf {
    let dirs = ProjectDirs::from("", "", "SistemaAcceso").expect("no se pudo obtener el dir de datos");
    let dir = dirs.data_local_dir().join("SistemaAcceso");
    std::fs::create_dir_all(&dir).expect("no se pudo crear el dir de datos");
    dir
}

pub fn db_path() -> PathBuf {
    data_dir().join("sistema_acceso.db")
}

pub fn abrir_db() -> Connection {
    let conn = Connection::open(db_path()).expect("no se pudo abrir la base de datos");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS estudiantes (
            matricula TEXT PRIMARY KEY,
            genero    TEXT NOT NULL,
            carrera   TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS accesos (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            matricula TEXT,
            sala      TEXT NOT NULL,
            carrera   TEXT NOT NULL,
            genero    TEXT NOT NULL,
            fecha     TEXT NOT NULL
        );",
    )
    .expect("no se pudo crear el esquema");
    conn
}

#[derive(Clone)]
pub struct Estudiante {
    pub matricula: String,
    pub genero: String,
    pub carrera: String,
}

pub fn normalizar_matricula(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).take(8).collect()
}

/// Devuelve la matrícula de 8 dígitos si es válida y está registrada.
pub fn matricula_valida(conn: &Connection, s: &str) -> Option<String> {
    let m = normalizar_matricula(s);
    if m.len() != 8 {
        return None;
    }
    conn
        .prepare("SELECT 1 FROM estudiantes WHERE matricula = ?1")
        .ok()?
        .exists(rusqlite::params![m])
        .ok()
        .filter(|ok| *ok)
        .map(|_| m)
}

pub fn estudiante(conn: &Connection, matricula: &str) -> Option<Estudiante> {
    conn.query_row(
        "SELECT matricula, genero, carrera FROM estudiantes WHERE matricula = ?1",
        rusqlite::params![matricula],
        |row| {
            Ok(Estudiante {
                matricula: row.get(0)?,
                genero: row.get(1)?,
                carrera: row.get(2)?,
            })
        },
    )
    .ok()
}

/// Devuelve false si la matrícula ya existe (no duplica).
pub fn insertar_estudiante(conn: &Connection, e: &Estudiante) -> Result<bool, rusqlite::Error> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO estudiantes (matricula, genero, carrera) VALUES (?1, ?2, ?3)",
        rusqlite::params![e.matricula, e.genero, e.carrera],
    )?;
    Ok(n > 0)
}

/// Acceso del kiosco: copia carrera/genero del estudiante, igual que el original.
pub fn insertar_acceso(conn: &Connection, matricula: &str, sala: &str) -> Result<(), String> {
    if matricula.len() != 8 {
        return Err("Matrícula debe tener 8 dígitos".into());
    }
    let e = estudiante(conn, matricula).ok_or_else(|| format!("Matrícula no registrada: {matricula}"))?;
    let fecha = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO accesos (matricula, sala, carrera, genero, fecha) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![matricula, sala, e.carrera, e.genero, fecha],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Acceso importado del formato antiguo: sin matrícula, con fecha sintética.
pub fn insertar_acceso_legacy(
    conn: &Connection,
    sala: &str,
    carrera: &str,
    genero: &str,
    fecha: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO accesos (matricula, sala, carrera, genero, fecha) VALUES (NULL, ?1, ?2, ?3, ?4)",
        rusqlite::params![sala, carrera, genero, fecha],
    )?;
    Ok(())
}

pub fn trimestre_de(fecha: &str) -> (i32, u32) {
    let dt = NaiveDateTime::parse_from_str(fecha, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDate::parse_from_str(fecha, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
        .expect("fecha inválida en la BD");
    let t = (dt.month() - 1) / 3 + 1;
    (dt.year(), t)
}

/// Último día del trimestre dado (fecha sintética para import de reportes viejos).
pub fn fin_de_trimestre(año: i32, trimestre: u32) -> NaiveDate {
    let fin_mes = match trimestre {
        1 => 3,
        2 => 6,
        3 => 9,
        _ => 12,
    };
    NaiveDate::from_ymd_opt(año, fin_mes + 1, 1)
        .and_then(|d| d.pred_opt())
        .expect("trimestre inválido")
}

pub fn rango_trimestre(año: i32, trimestre: u32) -> (String, String) {
    let inicio_mes = (trimestre - 1) * 3 + 1;
    let ini = NaiveDate::from_ymd_opt(año, inicio_mes, 1).unwrap();
    let fin = fin_de_trimestre(año, trimestre);
    (
        ini.format("%Y-%m-%d").to_string(),
        fin.format("%Y-%m-%d 23:59:59").to_string(),
    )
}

#[derive(Clone)]
pub struct ResumenFila {
    pub sala: String,
    pub carrera: String,
    pub masculino: i64,
    pub femenino: i64,
}

/// Agrupa accesos de un trimestre por (sala, carrera, genero), igual que el panel anterior.
pub fn resumen_trimestre(conn: &Connection, año: i32, trimestre: u32) -> Vec<ResumenFila> {
    let (ini, fin) = rango_trimestre(año, trimestre);
    let mut stmt = conn
        .prepare(
            "SELECT sala, carrera, genero, COUNT(*) AS n
             FROM accesos
             WHERE fecha BETWEEN ?1 AND ?2
             GROUP BY sala, carrera, genero
             ORDER BY sala, carrera",
        )
        .expect("consulta resumen");
    let filas = stmt
        .query_map(rusqlite::params![ini, fin], |row| {
            let genero: String = row.get(2)?;
            let n: i64 = row.get(3)?;
            Ok((genero, n, row.get::<_, String>(0)?.clone(), row.get::<_, String>(1)?.clone()))
        })
        .expect("leer resumen")
        .collect::<Result<Vec<_>, _>>()
        .expect("iterar resumen");

    let mut resumen: Vec<ResumenFila> = Vec::new();
    for (genero, n, sala, carrera) in filas {
        if let Some(f) = resumen.iter_mut().find(|f| f.sala == sala && f.carrera == carrera) {
            if genero == "Masculino" {
                f.masculino += n;
            } else {
                f.femenino += n;
            }
        } else {
            let mut f = ResumenFila {
                sala,
                carrera,
                masculino: 0,
                femenino: 0,
            };
            if genero == "Masculino" {
                f.masculino = n;
            } else {
                f.femenino = n;
            }
            resumen.push(f);
        }
    }
    resumen
}

pub fn periodos_disponibles(conn: &Connection) -> Vec<(i32, u32)> {
    let mut stmt = conn.prepare("SELECT fecha FROM accesos").expect("consulta periodos");
    let fechas: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("leer fechas")
        .collect::<Result<_, _>>()
        .unwrap_or_default();
    let mut periodos: Vec<(i32, u32)> = fechas.iter().map(|f| trimestre_de(f)).collect();
    periodos.sort_unstable();
    periodos.dedup();
    periodos
}

/// Detección de matrícula en texto de QR/escáner (misma lógica que el original).
pub fn detectar_matricula_en_codigo(conn: &Connection, codigo: &str) -> Option<String> {
    let limpiado: Vec<char> = codigo.replace(['\n', '\r'], " ").chars().collect();
    let mut i = 0;
    while i < limpiado.len() {
        if limpiado[i].is_ascii_digit() {
            let inicio = i;
            while i < limpiado.len() && limpiado[i].is_ascii_digit() {
                i += 1;
            }
            if i - inicio >= 8 {
                let posible: String = limpiado[inicio..inicio + 8].iter().collect();
                if matricula_valida(conn, &posible).is_some() {
                    return Some(posible);
                }
            }
        } else {
            i += 1;
        }
    }
    let digits: String = codigo.chars().filter(|c| c.is_ascii_digit()).take(8).collect();
    if digits.len() == 8 {
        matricula_valida(conn, &digits)
    } else {
        None
    }
}