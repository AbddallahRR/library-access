# Arquitectura del Sistema

Documentación técnica de la arquitectura interna del Sistema de Acceso Estudiantil.

## Visión general

La aplicación es un binario de escritorio compuesto por dos ventanas independientes que comparten una misma base de datos SQLite:

```
┌─────────────────────┐      ┌─────────────────────┐
│   Panel Admin       │      │   Kiosco Acceso     │
│   (bin: admin)      │      │   (bin: access)      │
│                     │      │                      │
│  ui::run_admin()    │      │  ui::run_access()    │
└─────────┬───────────┘      └──────────┬───────────┘
          │                             │
          ▼                             ▼
    ┌──────────────────────────────────────┐
    │          db.rs (SQLite)              │
    │   estudiantes  |  accesos           │
    └──────────────────────────────────────┘
```

## Módulos

### `db.rs` — Capa de datos

Responsabilidades:
- Crear/abrir la base de datos y asegurar el esquema (`abrir_db`)
- Validar matrículas contra la tabla `estudiantes` (`matricula_valida`)
- Insertar registros de acceso desde el kiosco (`insertar_acceso`)
- Insertar registros legacy sin matrícula (`insertar_acceso_legacy`)
- Generar resúmenes trimestrales agrupados por sala/carrera/género (`resumen_trimestre`)
- Detectar matrículas en texto crudo de escáner QR (`detectar_matricula_en_codigo`)

Constantes:
- `SALAS`: `["General", "Estudio", "Computo"]`

### `import.rs` — Importación de Excel

Responsabilidades:
- Leer lista de estudiantes desde `.xlsx` y validar cada fila (`leer_estudiantes_xlsx`)
- Insertar estudiantes válidos en la BD, ignorando duplicados (`importar_estudiantes`)
- Leer reportes históricos en formato antiguo (`leer_reporte_xlsx`)

### `ui.rs` — Lógica de presentación

Responsabilidades:
- `run_admin()`: Configura la ventana `AdminUI`, conecta callbacks de reportes, importación y popups.
- `run_access()`: Configura la ventana `AccessUI`, maneja entrada de matrícula, detección de QR, y registro de accesos.

Funciones auxiliares: `cargar_reporte()`, `mostrar_msg()`, `admitir()`, `reset_kiosco()`, `registrar()`.

### `ui/ui.slint` — Definición de UI

Componentes declarativos:
- `BotonRedondeado`: Botón circular con color, usado en el kiosco.
- `Boton`: Botón rectangular, usado en el panel admin.
- `MessagePopup`: Modal reutilizable para mensajes.
- `Tabla4`: Tabla genérica de 4 columnas con filas alternas.
- `Fila`: Struct exportada para pasar datos a la tabla.
- `AccessUI`: Ventana del kiosco (440×580).
- `AdminUI`: Ventana del administrador (1200×800).

## Flujo de datos

### Kiosco (registro de acceso)

```
1. Usuario escribe matrícula → entrada_cambiada()
2. detectar_matricula_en_codigo() busca 8 dígitos válidos
3. Si válida → admitir(): habilita botones, muestra matrícula
4. Usuario selecciona sala → registrar()
5. insertar_acceso(): copia carrera/género del estudiante, agrega timestamp
6. Muestra popup de confirmación → cerrar_msg() → reset_kiosco()
```

### Administración (reportes)

```
1. click "Cargar Reporte" → dialogo año/trimestre
2. confirmar_carga() → cargar_reporte()
3. resumen_trimestre(): agrupa accesos por (sala, carrera, género)
4. Muestra en Tabla4 con fila de totales
```

### Importación de estudiantes

```
1. click "Importar Estudiantes" → rfd file dialog
2. leer_estudiantes_xlsx(): lee, valida, marca errores
3. importar_estudiantes(): INSERT OR IGNORE en BD
4. Muestra popup con primeros/últimos 5 registros
```

## Hilos de ejecución

La aplicación es single-threaded. Slint maneja el event loop. Las operaciones de BD son síncronas (sin `async`), adecuadas para el volumen esperado de datos.

## Ubicación de la base de datos

En Linux: `~/.local/share/SistemaAcceso/SistemaAcceso/sistema_acceso.db`

La ruta se resuelve con el crate `directories` para ser cross-platform.
