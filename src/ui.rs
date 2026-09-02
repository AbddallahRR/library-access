use std::cell::RefCell;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel, Weak};

use crate::db;
use crate::import::{self, FilaEstudiante};

slint::include_modules!();

fn fila_estudiante_a_fila(f: &FilaEstudiante) -> Fila {
    Fila {
        c0: f.matricula.clone().into(),
        c1: f.genero.clone().into(),
        c2: f.carrera.clone().into(),
        c3: f.estado.clone().into(),
    }
}

fn filas_model(v: Vec<Fila>) -> ModelRc<Fila> {
    Rc::new(VecModel::from(v)).into()
}

fn filas_rc(v: Vec<SharedString>) -> ModelRc<SharedString> {
    Rc::new(VecModel::from(v)).into()
}

// ------------------------------------------------------------------ ADMIN

pub fn run_admin() {
    let conn = Rc::new(db::abrir_db());
    let ui = AdminUI::new().expect("no se pudo crear la UI");

    ui.on_cargar_reporte({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_dlg_año("".into());
                ui.set_dlg_trimestre("".into());
                ui.set_dlg_visible(true);
            }
        }
    });

    ui.on_cancelar_carga({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_dlg_visible(false);
            }
        }
    });

    ui.on_confirmar_carga({
        let ui = ui.as_weak();
        let conn = conn.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_dlg_visible(false);
                let (y, t) = (
                    ui.get_dlg_año().trim().parse::<i32>(),
                    ui.get_dlg_trimestre().trim().parse::<u32>(),
                );
                match (y, t) {
                    (Ok(año), Ok(trim)) if (1..=4).contains(&trim) => cargar_reporte(&ui, &conn, año, trim),
                    _ => mostrar_msg(&ui, "Error", "Año o trimestre inválidos"),
                }
            }
        }
    });

    ui.on_reportes_disponibles({
        let ui = ui.as_weak();
        let conn = conn.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                let periodos = db::periodos_disponibles(&conn);
                if periodos.is_empty() {
                    mostrar_msg(&ui, "Info", "No hay reportes disponibles");
                    return;
                }
                let items: Vec<SharedString> = periodos
                    .iter()
                    .map(|(a, t)| format!("Año {a} - Trimestre {t}").into())
                    .collect();
                ui.set_rep_items(filas_rc(items));
                ui.set_rep_visible(true);
            }
        }
    });

    ui.on_cargar_periodo({
        let ui = ui.as_weak();
        let conn = conn.clone();
        move |item: SharedString| {
            if let Some(ui) = ui.upgrade() {
                ui.set_rep_visible(false);
                let p: Vec<&str> = item.trim().split_whitespace().collect();
                if p.len() == 5 {
                    if let (Ok(año), Ok(trim)) = (p[1].parse::<i32>(), p[4].parse::<u32>()) {
                        cargar_reporte(&ui, &conn, año, trim);
                    }
                }
            }
        }
    });

    ui.on_cerrar_reportes({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_rep_visible(false);
            }
        }
    });

    ui.on_importar_estudiantes({
        let ui = ui.as_weak();
        let conn = conn.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                let archivo = rfd::FileDialog::new()
                    .add_filter("Excel", &["xlsx", "xls"])
                    .pick_file();
                let Some(archivo) = archivo else { return };
                match import::leer_estudiantes_xlsx(&archivo) {
                    Err(e) => mostrar_msg(&ui, "Error", &e),
                    Ok(filas) => {
                        if filas.is_empty() {
                            mostrar_msg(&ui, "Advertencia", "No hay matrículas válidas (8 dígitos)");
                            return;
                        }
                        let (exitos, errores, resultado) = import::importar_estudiantes(&conn, filas);
                        let (primeros, ultimos) = import::primeros_ultimos(&resultado, 5);
                        ui.set_imp_primeros(filas_model(primeros.iter().map(fila_estudiante_a_fila).collect()));
                        ui.set_imp_ultimos(filas_model(ultimos.iter().map(fila_estudiante_a_fila).collect()));
                        ui.set_imp_totales(totales(&resultado, exitos, errores).into());
                        ui.set_imp_visible(true);
                    }
                }
            }
        }
    });

    ui.on_cerrar_import({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_imp_visible(false);
            }
        }
    });

    ui.on_importar_reportes_viejos({
        let ui = ui.as_weak();
        let conn = conn.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                let archivos = rfd::FileDialog::new()
                    .add_filter("Excel", &["xlsx"])
                    .pick_files();
                let Some(archivos) = archivos else { return };
                let mut total = 0usize;
                let mut errores = 0usize;
                let tx = conn.unchecked_transaction().expect("iniciar transacción");
                for archivo in &archivos {
                    match import::leer_reporte_xlsx(archivo) {
                        Ok((_, _, accesos)) => {
                            for acc in &accesos {
                                if db::insertar_acceso_legacy(&tx, &acc.sala, &acc.carrera, &acc.genero, &acc.fecha).is_ok() {
                                    total += 1;
                                } else {
                                    errores += 1;
                                }
                            }
                        }
                        Err(e) => {
                            errores += 1;
                            eprintln!("{e}");
                        }
                    }
                }
                tx.commit().expect("confirmar transacción");
                mostrar_msg(&ui, "Importación completa", &format!("Accesos importados: {total}\nErrores: {errores}"));
            }
        }
    });

    ui.on_cerrar_msg({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_msg_visible(false);
            }
        }
    });

    ui.run().expect("error en el bucle de eventos");
}

fn totales(resultado: &[FilaEstudiante], exitos: usize, errores: usize) -> String {
    format!(
        "Total de filas: {}     —     Éxito: {}     —     Errores: {}",
        resultado.len(),
        exitos,
        errores
    )
}

fn cargar_reporte(ui: &AdminUI, conn: &rusqlite::Connection, año: i32, trimestre: u32) {
    let resumen = db::resumen_trimestre(conn, año, trimestre);
    if resumen.is_empty() {
        mostrar_msg(ui, "Vacío", &format!("No hay datos para mostrar en el reporte {año}-T{trimestre}"));
        ui.set_hay_reporte(false);
        return;
    }
    let mut filas: Vec<Fila> = resumen
        .iter()
        .map(|f| Fila {
            c0: f.sala.clone().into(),
            c1: f.carrera.clone().into(),
            c2: f.masculino.to_string().into(),
            c3: f.femenino.to_string().into(),
        })
        .collect();
    let total_m: i64 = resumen.iter().map(|f| f.masculino).sum();
    let total_f: i64 = resumen.iter().map(|f| f.femenino).sum();
    filas.push(Fila {
        c0: "TOTAL".into(),
        c1: "".into(),
        c2: total_m.to_string().into(),
        c3: total_f.to_string().into(),
    });
    ui.set_filas_reporte(filas_model(filas));
    ui.set_cabeceras_reporte(filas_rc(vec!["Sala".into(), "Carrera".into(), "Masculino".into(), "Femenino".into()]));
    ui.set_hay_reporte(true);
}

fn mostrar_msg(ui: &AdminUI, titulo: &str, texto: &str) {
    ui.set_msg_titulo(titulo.into());
    ui.set_msg_texto(texto.into());
    ui.set_msg_visible(true);
}

// ------------------------------------------------------------------ KIOSCO

type EstadoMatricula = Rc<RefCell<Option<String>>>;

fn admitir(ui: &AccessUI, mat: &EstadoMatricula, matricula: &str) {
    mat.borrow_mut().replace(matricula.to_string());
    ui.set_label_info("".into());
    ui.set_qr_info(format!("Matrícula detectada: {matricula}").into());
    ui.set_buttons_enabled(true);
    ui.set_entrada_visible(false);
    ui.set_entrada("".into());
}

fn reset_kiosco(ui: &AccessUI, mat: &EstadoMatricula) {
    mat.borrow_mut().take();
    ui.set_buttons_enabled(false);
    ui.set_entrada_visible(true);
    ui.set_entrada("".into());
    ui.set_label_info("Escribe tu matrícula o escanea QR:".into());
    ui.set_qr_info("".into());
    ui.set_msg_visible(false);
}

fn mostrar_msg_k(ui: &AccessUI, titulo: &str, texto: &str) {
    ui.set_msg_titulo(titulo.into());
    ui.set_msg_texto(texto.into());
    ui.set_msg_visible(true);
}

fn registrar(ui: &AccessUI, conn: &rusqlite::Connection, mat: &EstadoMatricula, sala: &str) {
    let Some(m) = mat.borrow().clone() else {
        mostrar_msg_k(ui, "Error", "Matrícula no registrada");
        return;
    };
    match db::insertar_acceso(conn, &m, sala) {
        Ok(e) => {
            let texto = format!(
                "Acceso registrado exitosamente:\n\nMatrícula: {}\nCarrera: {}\nGénero: {}\nSala: {}",
                e.matricula, e.carrera, e.genero, sala
            );
            mostrar_msg_k(ui, "Éxito", &texto);
        }
        Err(e) => mostrar_msg_k(ui, "Error", &e),
    }
}

pub fn run_access() {
    let conn = Rc::new(db::abrir_db());
    let ui = AccessUI::new().expect("no se pudo crear la UI");
    let mat: EstadoMatricula = Rc::new(RefCell::new(None));

    ui.on_entrada_cambiada({
        let ui = ui.as_weak();
        let conn = conn.clone();
        let mat = mat.clone();
        move |texto: SharedString| {
            let Some(ui) = ui.upgrade() else { return };
            if let Some(m) = db::detectar_matricula_en_codigo(&conn, &texto) {
                admitir(&ui, &mat, &m);
            }
        }
    });

    ui.on_solicitar_escaneo({
        let ui = ui.as_weak();
        let conn = conn.clone();
        let mat = mat.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let texto = ui.get_entrada();
            if let Some(m) = db::detectar_matricula_en_codigo(&conn, &texto) {
                admitir(&ui, &mat, &m);
            } else {
                mostrar_msg_k(&ui, "Error", "Matrícula inválida o no registrada.");
                ui.set_entrada("".into());
            }
        }
    });

    let ui_weak: Weak<AccessUI> = ui.as_weak();
    ui.on_sala_general({
        let ui = ui_weak.clone();
        let conn = conn.clone();
        let mat = mat.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                registrar(&ui, &conn, &mat, "General");
            }
        }
    });
    ui.on_sala_estudio({
        let ui = ui_weak.clone();
        let conn = conn.clone();
        let mat = mat.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                registrar(&ui, &conn, &mat, "Estudio");
            }
        }
    });
    ui.on_sala_computo({
        let ui = ui_weak.clone();
        let conn = conn.clone();
        let mat = mat.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                registrar(&ui, &conn, &mat, "Computo");
            }
        }
    });
    ui.on_cerrar_msg({
        let ui = ui_weak.clone();
        let mat = mat.clone();
        move || {
            if let Some(ui) = ui.upgrade() {
                reset_kiosco(&ui, &mat);
            }
        }
    });

    ui.run().expect("error en el bucle de eventos");
}