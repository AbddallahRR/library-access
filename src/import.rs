use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

use crate::db::{self, Estudiante};

#[derive(Clone)]
pub struct FilaEstudiante {
    pub matricula: String,
    pub genero: String,
    pub carrera: String,
    pub estado: String,
}

/// Lee estudiantes de un .xlsx (columnas numero_control/number, genero, carrera).
/// Devuelve las filas con status; las inválidas se marcan como error y no se insertan.
pub fn leer_estudiantes_xlsx(path: &Path) -> Result<Vec<FilaEstudiante>, String> {
    let mut libro = open_workbook_auto(path).map_err(|e| format!("No se pudo abrir: {e}"))?;
    let hoja = libro
        .worksheet_range_at(0)
        .ok_or("El archivo no tiene hojas")?
        .map_err(|e| format!("Error leyendo hoja: {e}"))?;

    let mut it = hoja.rows();
    let header = it.next().ok_or("El archivo está vacío")?;
    let idx = |nombre: &str| -> Option<usize> {
        header
            .iter()
            .position(|c| matches!(c, Data::String(s) if s.trim().eq_ignore_ascii_case(nombre)))
    };
    let i_mat = idx("numero_control")
        .or_else(|| idx("matricula"))
        .or_else(|| idx("num_control"))
        .ok_or("No se encontró la columna de matrícula")?;
    let i_gen = idx("genero").ok_or("No se encontró la columna de género")?;
    let i_car = idx("carrera").ok_or("No se encontró la columna de carrera")?;

    let cell = |v: &Data| -> String {
        match v {
            Data::String(s) => s.trim().to_string(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                let s = format!("{f}");
                s.trim_end_matches(".0").to_string()
            }
            Data::DateTime(d) => d.as_f64().to_string(),
            _ => String::new(),
        }
    };

    let mut filas = Vec::new();
    for row in it {
        let raw_matricula = row.get(i_mat).map(cell).unwrap_or_default();
        if raw_matricula.is_empty() {
            continue;
        }
        let matricula = db::normalizar_matricula(&raw_matricula);
        let genero_raw = row.get(i_gen).map(cell).unwrap_or_default();
        let genero = genero_raw.trim().to_string();
        let carrera = {
            let c = row.get(i_car).map(cell).unwrap_or_default();
            c.split('/').next().unwrap_or("").trim().to_string()
        };

        let estado = if matricula.len() != 8 {
            format!("Error: matrícula inválida ({raw_matricula})")
        } else if genero != "Masculino" && genero != "Femenino" {
            format!("Error: género inválido en {matricula}")
        } else {
            String::new()
        };
        filas.push(FilaEstudiante {
            matricula: matricula.clone(),
            genero: genero.clone(),
            carrera,
            estado,
        });
    }
    Ok(filas)
}

/// Inserta las filas válidas; devuelve conteos y estado final de cada fila.
pub fn importar_estudiantes(
    conn: &rusqlite::Connection,
    filas: Vec<FilaEstudiante>,
) -> (usize, usize, Vec<FilaEstudiante>) {
    let mut exitos = 0;
    let mut errores = 0;
    let mut resultado = Vec::with_capacity(filas.len());
    for mut f in filas {
        if !f.estado.is_empty() {
            errores += 1;
            resultado.push(f);
            continue;
        }
        let e = Estudiante {
            matricula: f.matricula.clone(),
            genero: f.genero.clone(),
            carrera: f.carrera.clone(),
        };
        match db::insertar_estudiante(conn, &e) {
            Ok(true) => {
                exitos += 1;
                f.estado = "Éxito: agregado".to_string();
            }
            Ok(false) => {
                errores += 1;
                f.estado = format!("Error: {} ya existe", f.matricula);
            }
            Err(_) => {
                errores += 1;
                f.estado = format!("Error: fallo al insertar {}", f.matricula);
            }
        }
        resultado.push(f);
    }
    (exitos, errores, resultado)
}

#[derive(Clone)]
pub struct AccesoLegacy {
    pub sala: String,
    pub carrera: String,
    pub genero: String,
    pub fecha: String,
}

/// Lee un reporte trimestral viejo (Reporte_AÑO_TN.xlsx, columnas sala/carrera/genero)
/// y le asigna la fecha sintética de fin de trimestre.
pub fn leer_reporte_xlsx(path: &Path) -> Result<(i32, u32, Vec<AccesoLegacy>), String> {
    let nombre = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Nombre de archivo inválido")?;
    let base = nombre.strip_suffix(".xlsx").unwrap_or(nombre);
    let partes: Vec<&str> = base.split('_').collect();
    if partes.len() != 3 {
        return Err(format!("Nombre no reconocido: {nombre} (esperado Reporte_AÑO_TN)"));
    }
    let año: i32 = partes[1].parse().map_err(|_| format!("Año inválido en {nombre}"))?;
    let t_str = partes[2];
    if !t_str.starts_with('T') {
        return Err(format!("Trimestre inválido en {nombre}"));
    }
    let trimestre: u32 = t_str[1..].parse().map_err(|_| format!("Trimestre inválido en {nombre}"))?;
    if !(1..=4).contains(&trimestre) {
        return Err(format!("Trimestre fuera de rango en {nombre}"));
    }

    let mut libro = open_workbook_auto(path).map_err(|e| format!("No se pudo abrir: {e}"))?;
    let hoja = libro
        .worksheet_range_at(0)
        .ok_or("El archivo no tiene hojas")?
        .map_err(|e| format!("Error leyendo hoja: {e}"))?;

    let fin = db::fin_de_trimestre(año, trimestre).format("%Y-%m-%d").to_string();
    let mut accesos = Vec::new();
    for row in hoja.rows().skip(1) {
        if row.len() < 3 {
            continue;
        }
        let celda = |i: usize| -> String {
            match &row[i] {
                Data::String(s) => s.trim().to_string(),
                Data::Int(n) => n.to_string(),
                _ => String::new(),
            }
        };
        let (sala, carrera, genero) = (celda(0), celda(1), celda(2));
        let genero = {
            if genero.is_empty() {
                continue;
            }
            let mut c = genero.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => continue,
            }
        };
        accesos.push(AccesoLegacy { sala, carrera, genero, fecha: fin.clone() });
    }
    Ok((año, trimestre, accesos))
}

/// Primeros y últimos n elementos de una lista (en orden original).
pub fn primeros_ultimos<T: Clone>(lista: &[T], n: usize) -> (Vec<T>, Vec<T>) {
    let primeros = lista.iter().take(n).cloned().collect();
    let ultimos = lista
        .iter()
        .skip(lista.len().saturating_sub(n))
        .cloned()
        .collect();
    (primeros, ultimos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primeros_ultimos_5() {
        let v: Vec<i32> = (1..=20).collect();
        let (p, u) = primeros_ultimos(&v, 5);
        assert_eq!(p, vec![1, 2, 3, 4, 5]);
        assert_eq!(u, vec![16, 17, 18, 19, 20]);
    }

    #[test]
    fn primeros_ultimos_menos_de_n() {
        let v = vec![1, 2, 3];
        let (p, u) = primeros_ultimos(&v, 5);
        assert_eq!(p, vec![1, 2, 3]);
        assert_eq!(u, vec![1, 2, 3]);
    }

    #[test]
    fn fila_invalida_queda_marcada() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE estudiantes (matricula TEXT PRIMARY KEY, genero TEXT, carrera TEXT);",
        )
        .unwrap();
        let filas = vec![
            FilaEstudiante { matricula: "10022001".into(), genero: "Masculino".into(), carrera: "Sistemas".into(), estado: String::new() },
            FilaEstudiante { matricula: "123".into(), genero: "Masculino".into(), carrera: "Sistemas".into(), estado: "Error: matrícula inválida (123)".into() },
        ];
        let (exitos, errores, _) = importar_estudiantes(&mut conn, filas);
        assert_eq!(exitos, 1);
        assert_eq!(errores, 1);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM estudiantes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}