# Esquema de Base de Datos

## SQLite

La base de datos se crea automáticamente en `abrir_db()` y reside en:
- **Linux:** `~/.local/share/SistemaAcceso/SistemaAcceso/sistema_acceso.db`
- **macOS:** `~/Library/Application Support/SistemaAcceso/SistemaAcceso/sistema_acceso.db`
- **Windows:** `C:\Users\<usuario>\AppData\Local\SistemaAcceso\SistemaAcceso\sistema_acceso.db`

## Tablas

### `estudiantes`

Almacena la información de cada estudiante registrado.

```sql
CREATE TABLE IF NOT EXISTS estudiantes (
    matricula TEXT PRIMARY KEY,  -- Matrícula de 8 dígitos
    genero    TEXT NOT NULL,     -- "Masculino" o "Femenino"
    carrera   TEXT NOT NULL      -- Nombre de la carrera
);
```

| Columna | Tipo | Constraints | Descripción |
|---------|------|-------------|-------------|
| `matricula` | TEXT | PRIMARY KEY | Identificador único de 8 dígitos numéricos |
| `genero` | TEXT | NOT NULL | Solo se acepta `Masculino` o `Femenino` |
| `carrera` | TEXT | NOT NULL | Nombre de la carrera (se trunca en `/`) |

### `accesos`

Registra cada entrada de un estudiante a una sala.

```sql
CREATE TABLE IF NOT EXISTS accesos (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    matricula TEXT,
    sala      TEXT NOT NULL,      -- "General", "Estudio" o "Computo"
    carrera   TEXT NOT NULL,      -- Copiado del estudiante al momento del acceso
    genero    TEXT NOT NULL,      -- Copiado del estudiante al momento del acceso
    fecha     TEXT NOT NULL       -- Formato: "YYYY-MM-DD HH:MM:SS"
);
```

| Columna | Tipo | Constraints | Descripción |
|---------|------|-------------|-------------|
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | ID autoincremental |
| `matricula` | TEXT | NULL permitido | Matrícula del estudiante (NULL en accesos legacy) |
| `sala` | TEXT | NOT NULL | Sala: `General`, `Estudio` o `Computo` |
| `carrera` | TEXT | NOT NULL | Carrera del estudiante al momento del acceso |
| `genero` | TEXT | NOT NULL | Género del estudiante al momento del acceso |
| `fecha` | TEXT | NOT NULL | Fecha/hora del acceso |

> **Nota:** Los registros importados del formato antiguo (legacy) tienen `matricula = NULL` y una fecha sintética correspondiente al último día del trimestre.

## Consultas principales

### Resumen trimestral

```sql
SELECT sala, carrera, genero, COUNT(*) AS n
FROM accesos
WHERE fecha BETWEEN ?1 AND ?2
GROUP BY sala, carrera, genero
ORDER BY sala, carrera;
```

Devuelve la cantidad de accesos agrupados por sala, carrera y género para un rango de fechas (trimestre).

### Validación de matrícula

```sql
SELECT 1 FROM estudiantes WHERE matricula = ?1;
```

### Periodos disponibles

```sql
SELECT DISTINCT fecha FROM accesos;
```

Se procesa en Rust para extraer `(año, trimestre)` únicos.

## Convenciones de fechas

- **Kiosco:** `chrono::Local::now()` formateado como `%Y-%m-%d %H:%M:%S`.
- **Legacy:** Último día del trimestre (ej. T1 → `2024-03-31`).
- **Rango trimestre:** Desde el primer día del mes de inicio hasta el último día del trimestre.

| Trimestre | Mes inicio | Mes fin |
|-----------|-----------|---------|
| T1 | Enero (1) | Marzo (3) |
| T2 | Abril (4) | Junio (6) |
| T3 | Julio (7) | Septiembre (9) |
| T4 | Octubre (10) | Diciembre (12) |
