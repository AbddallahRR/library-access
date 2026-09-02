//! Benchmark headless de la capa de datos (sin UI).
//! Validar los cambios de rendimiento: correr con `cargo run --release --example bench`.
use std::time::Instant;

use rusqlite::Connection;

use sistema_acceso::db::{self, Estudiante};

fn seed(conn: &Connection, estudiantes: usize, accesos: usize) {
    conn.execute_batch("BEGIN;").unwrap();
    for i in 0..estudiantes {
        let matricula = format!("{:08}", i);
        let genero = if i % 2 == 0 { "Masculino" } else { "Femenino" };
        conn.execute(
            "INSERT OR IGNORE INTO estudiantes (matricula, genero, carrera) VALUES (?1, ?2, ?3)",
            rusqlite::params![matricula, genero, "Ingeniería en Sistemas"],
        )
        .unwrap();
        if i % 20 == 0 {
            // acceso esparcido en varios trimestres
            let fecha = format!("202{:01}-{:02}-15 10:00:00", (i / 2) % 9, (i % 12) + 1);
            conn.execute(
                "INSERT INTO accesos (matricula, sala, carrera, genero, fecha) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![matricula, "General", "Ingeniería en Sistemas", genero, fecha],
            )
            .unwrap();
        }
    }
    conn.execute_batch("COMMIT;").unwrap();
    // completar hasta `accesos` con fechas sintéticas (legacy, sin matrícula)
    conn.execute_batch("BEGIN;").unwrap();
    for i in 0..accesos {
        let sala = db::SALAS[i % db::SALAS.len()];
        let fecha = format!("202{:01}-{:02}-{:02} 10:00:00", (i / 120) % 9, (i / 10) % 12 + 1, i % 28 + 1);
        conn.execute(
            "INSERT INTO accesos (matricula, sala, carrera, genero, fecha) VALUES (NULL, ?1, ?2, ?3, ?4)",
            rusqlite::params![sala, "Ingeniería en Sistemas", "Masculino", fecha],
        )
        .unwrap();
    }
    conn.execute_batch("COMMIT;").unwrap();
}

fn main() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE estudiantes (matricula TEXT PRIMARY KEY, genero TEXT, carrera TEXT);
         CREATE TABLE accesos (id INTEGER PRIMARY KEY AUTOINCREMENT, matricula TEXT,
             sala TEXT, carrera TEXT, genero TEXT, fecha TEXT);
         CREATE INDEX IF NOT EXISTS idx_accesos_fecha ON accesos(fecha);",
    )
    .unwrap();

    let (n_est, n_acc) = (5000usize, 100_000usize);
    let t = Instant::now();
    seed(&conn, n_est, n_acc);
    eprintln!("seed: {} estudiantes, {} accesos en {:?}", n_est, n_acc, t.elapsed());

    let t = Instant::now();
    let r = db::resumen_trimestre(&conn, 2024, 1);
    eprintln!("resumen_trimestre(2024,T1): {} filas en {:?}", r.len(), t.elapsed());

    let t = Instant::now();
    let p = db::periodos_disponibles(&conn);
    eprintln!("periodos_disponibles: {} periodos en {:?}", p.len(), t.elapsed());

    let t = Instant::now();
    let mut i = 0usize;
    for _ in 0..1000 {
        let matricula = format!("{:08}", i % n_est);
        if db::insertar_acceso(&conn, &matricula, db::SALAS[i % 3]).is_ok() {
            i += 1;
        }
    }
    eprintln!("insertar_acceso x {i} en {:?}", t.elapsed());

    // importar_estudiantes con transacción (un lote de 5000)
    let mut filas = Vec::new();
    for i in 10_000..15_000 {
        filas.push(sistema_acceso::import::FilaEstudiante {
            matricula: format!("{:08}", i),
            genero: "Masculino".into(),
            carrera: "Electrónica".into(),
            estado: String::new(),
        });
    }
    let t = Instant::now();
    let (ok, err, _) = sistema_acceso::import::importar_estudiantes(&conn, filas);
    eprintln!("importar_estudiantes ({} filas): {} ok, {} err en {:?}", 5000, ok, err, t.elapsed());

    assert!(db::matricula_valida(&conn, "00000000").is_some());
    assert!(db::insertar_acceso(&conn, "00000000x", "General").is_err());

    let e = Estudiante { matricula: "10000000".into(), genero: "Femenino".into(), carrera: "Química".into() };
    assert!(db::insertar_estudiante(&conn, &e).unwrap());
    assert_eq!(db::insertar_estudiante(&conn, &e).unwrap(), false);
    eprintln!("checks unitarios de db: OK");
}
