slint::include_modules!();

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{PhysicalSize, VecModel};
use std::rc::Rc as StdRc;

#[derive(Clone, Copy, Default)]
struct Rgb(u8, u8, u8);

impl TargetPixel for Rgb {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        *self = Rgb(c.red, c.green, c.blue);
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Rgb(r, g, b)
    }
}

struct TestPlatform(StdRc<MinimalSoftwareWindow>);

impl Platform for TestPlatform {
    fn create_window_adapter(&self) -> Result<StdRc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

fn snapshot(out: &str, ui: &AdminUI, window: &StdRc<MinimalSoftwareWindow>) {
    let w = 1200u32;
    let h = 800u32;
    window.set_size(PhysicalSize::new(w, h));
    let _ = ui.show();
    window.request_redraw();
    let mut buf = vec![Rgb(0, 0, 0); (w * h) as usize];
    let ok = window.draw_if_needed(|r| { r.render(buf.as_mut_slice(), w as usize); });
    eprintln!("{out}: draw_if_needed={ok}");
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in &buf {
        ppm.extend_from_slice(&[p.0, p.1, p.2]);
    }
    std::fs::write(out, ppm).unwrap();
}

fn filas_model(v: Vec<Fila>) -> slint::ModelRc<Fila> {
    StdRc::new(VecModel::from(v)).into()
}

fn main() {
    let window =
        MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let _ = slint::platform::set_platform(Box::new(TestPlatform(window.clone())));

    let ui = AdminUI::new().unwrap();
    let mut filas = Vec::new();
    for i in 0..8 {
        filas.push(Fila {
            c0: format!("Sala {i}").into(),
            c1: "Ingeniería en Sistemas Computacionales".into(),
            c2: format!("{}", 20 + i).into(),
            c3: format!("{}", 10 + i).into(),
        });
    }
    ui.set_filas_reporte(filas_model(filas));
    let cab: Vec<slint::SharedString> = vec![
        "Sala".into(),
        "Carrera".into(),
        "Masculino".into(),
        "Femenino".into(),
    ];
    ui.set_cabeceras_reporte(StdRc::new(VecModel::from(cab.clone())).into());
    ui.set_hay_reporte(true);
    snapshot("/tmp/opencode/report_short.ppm", &ui, &window);
    drop(ui);

    let ui = AdminUI::new().unwrap();
    let mut filas = Vec::new();
    for i in 0..60 {
        filas.push(Fila {
            c0: format!("Sala {i}").into(),
            c1: "Ingeniería en Sistemas Computacionales".into(),
            c2: format!("{}", 300 + i).into(),
            c3: format!("{}", 200 + i).into(),
        });
    }
ui.set_filas_reporte(filas_model(filas));
    ui.set_cabeceras_reporte(StdRc::new(VecModel::from(cab)).into());
    ui.set_hay_reporte(true);
    snapshot("/tmp/opencode/report_long.ppm", &ui, &window);
    drop(ui);

    window.set_size(PhysicalSize::new(440, 580));
    let ui = AccessUI::new().unwrap();
    ui.set_msg_visible(true);
    ui.set_msg_titulo("Éxito".into());
    ui.set_msg_texto("Acceso registrado exitosamente:\nSala: General".into());
    let w = 440u32;
    let h = 580u32;
    let _ = ui.show();
    window.request_redraw();
    let mut buf = vec![Rgb(0, 0, 0); (w * h) as usize];
    window.draw_if_needed(|r| {
        r.render(buf.as_mut_slice(), w as usize);
    });
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in &buf {
        ppm.extend_from_slice(&[p.0, p.1, p.2]);
    }
    std::fs::write("/tmp/opencode/access.ppm", ppm).unwrap();
}