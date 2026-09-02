# Sistema de Acceso Estudiantil

Aplicación de escritorio para el control de acceso de estudiantes a salas de biblioteca del Tecnológico Nacional de México. Registra entradas por sala, carrera y género, y genera reportes trimestrales.

## Características

- **Kiosco de acceso:** Los estudiantes escriben o escanean su matrícula (8 dígitos) y seleccionan la sala a la que ingresan.
- **Panel de administración:** Visualización de reportes trimestrales, importación de listas de estudiantes desde Excel, e importación de reportes históricos.
- **Base de datos local:** SQLite sin servidor, se crea automáticamente en el primer uso.
- **Sin dependencias de red:** Todo funciona en local.

## Requisitos previos

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- Compiladores del sistema para SQLite (bundled):
  - **Linux:** `gcc`, `make`, `pkg-config`
  - **macOS:** Xcode Command Line Tools
  - **Windows:** MSVC (Visual Studio Build Tools)

## Instalación y ejecución

```bash
# Clonar el repositorio
git clone <url-del-repositorio>
cd Sistema-Acceso

# Compilar (debug)
cargo build

# Compilar (release, optimizado)
cargo build --release

# Ejecutar panel de administración
cargo run --bin admin

# Ejecutar kiosco de acceso
cargo run --bin access
```

## Uso

### Panel de administración

1. **Importar estudiantes:** Click en "Importar Estudiantes (xlsx)" y selecciona el archivo Excel.
2. **Cargar reporte:** Click en "Cargar Reporte", ingresa año y trimestre (1-4).
3. **Ver reportes disponibles:** Click en "Reportes Disponibles" para ver periodos con datos.
4. **Importar reportes antiguos:** Click en "Importar Reportes Antiguos (xlsx)" para migrar datos históricos.

### Kiosco de acceso

1. El estudiante escribe su matrícula en el campo de texto o escanea un código QR.
2. Al detectarse una matrícula válida, se habilitan los botones de sala.
3. Selecciona la sala: **General**, **Estudio** o **Computo**.
4. Se muestra un mensaje de confirmación con los datos del estudiante.

### Formato de importación de estudiantes

El archivo Excel debe contener las columnas:

| Columna | Descripción |
|---------|-------------|
| `numero_control` / `matricula` / `num_control` | Matrícula de 8 dígitos |
| `genero` | `Masculino` o `Femenino` |
| `carrera` | Nombre de la carrera |

### Formato de reportes antiguos

Los archivos deben llamarse `Reporte_AÑO_TN.xlsx` (ej. `Reporte_2024_T1.xlsx`) y contener columnas: sala, carrera, género.

## Estructura del proyecto

```
Sistema-Acceso/
├── Cargo.toml              # Manifiesto del crate
├── build.rs                # Compila la UI de Slint en tiempo de build
├── imagenes/
│   └── logo_tecnm.png      # Logo institucional
├── src/
│   ├── lib.rs              # Raíz de la librería
│   ├── main.rs             # Binario "admin"
│   ├── bin/
│   │   └── access.rs       # Binario "access" (kiosco)
│   ├── db.rs               # Capa de base de datos (SQLite)
│   ├── import.rs           # Lógica de importación desde Excel
│   ├── ui.rs               # Lógica de UI y eventos
│   └── ui/
│       └── ui.slint        # Definición declarativa de la UI
└── examples/
    └── render_report.rs    # Renderizado headless de la UI
```

## Tecnologías

| Capa | Tecnología |
|------|------------|
| Lenguaje | Rust (edición 2021) |
| UI | [Slint](https://slint.dev) v1.17 |
| Base de datos | SQLite via `rusqlite` (bundled) |
| Excel | `calamine` v0.36 |
| Diálogos de archivo | `rfd` v0.17 |
| Fechas | `chrono` v0.4 |

## Testing

```bash
cargo test
```

## Licencia

Consultar el archivo `LICENSE` del repositorio.
