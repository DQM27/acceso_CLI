# Diagrama lógico de Control de Acceso

Este documento describe el funcionamiento interno de la aplicación. Omite el dibujo de
pantallas, colores y distribución visual de la TUI.

## 1. Flujo general

```mermaid
flowchart LR
    U["Usuario"] --> K["Evento de teclado"]
    K --> S["Estado de la vista<br/>handle_key"]
    S --> A["Acción de aplicación<br/>buscar, crear, actualizar o registrar"]
    A --> C["AppCore<br/>fachada y propietaria de la conexión"]

    C -->|Comandos| SV["Servicios de negocio"]
    C -->|Lecturas| CQ["Servicios de consulta"]

    SV --> D["Dominio puro<br/>acceso y cronología"]
    SV --> R["Repositorios SQLite"]
    CQ --> Q["Queries y read models"]

    R --> DB[("SQLite")]
    Q --> DB

    DB --> X["Resultado o error tipado"]
    D --> X
    X --> P["App procesa el resultado"]
    P --> S
```

El patrón de cada operación es:

```text
tecla → State.handle_key() → Accion* → App → AppCore
      → servicio/query → repositorio/SQLite → Result
      → State.completar_*() → nuevo estado
```

- Los estados de la TUI no ejecutan SQL.
- `AppCore` conserva la única conexión y compone servicios, repositorios y consultas para
  cada caso de uso; no contiene reglas de negocio.
- Las llamadas son directas, síncronas y ocurren en el mismo bucle de eventos.

## 2. Arranque, configuración y sesión

```mermaid
flowchart TD
    A["main()"] --> B["Resolver ruta de la base<br/>CONTROL_ACCESO_DB o LOCALAPPDATA"]
    B --> BL["Adquirir bloqueo de instancia<br/>para esa base"]
    BL --> C["Abrir SQLite"]
    C --> D["Activar claves foráneas<br/>y migrar dentro de una sola<br/>transacción IMMEDIATE"]
    D --> E["Crear AppCore"]
    E --> F{"¿La tabla usuarios está vacía?"}

    F -->|Sí| G["Solicitar ROOT inicial"]
    G --> H["Validar campos y contraseña mínima de 8 caracteres"]
    H -->|Error| G
    H -->|Válido| I["Generar hash Argon2"]
    I --> J["Crear ROOT activo en transacción IMMEDIATE"]
    J -->|Error| G
    J -->|Éxito o ya fue creado| L["Login"]

    F -->|No| L
    L --> M["Buscar usuario por cédula normalizada"]
    M --> N{"¿Existe y está activo?"}
    N -->|No| L
    N -->|Sí| O{"¿Contraseña válida?"}
    O -->|No| L
    O -->|Sí| P["Crear UsuarioSesion<br/>id, cédula, nombre y rol"]
    P --> Q["Menú principal"]

    Q --> R["Nuevo ingreso"]
    Q --> S["Ingresos activos"]
    Q --> T["Historial"]
    Q --> V["Administrar contratistas"]
    Q --> W["Administrar empresas"]
    Q --> X["Administrar usuarios"]

    R -->|Volver| Q
    S -->|Volver| Q
    T -->|Volver| Q
    V -->|Volver| Q
    W -->|Volver| Q
    X -->|Volver| Q
    Q -->|Cerrar sesión| L
    Q -->|Salir| Z["Restaurar terminal y finalizar"]
```

El inicio ROOT es atómico: dos instancias no pueden crear simultáneamente dos usuarios
iniciales. La autenticación nunca devuelve el `password_hash` dentro de la sesión.

## 3. Registro de una entrada

La preparación muestra el estado actual, pero no reserva recursos ni funciona como una
autorización guardada. Al confirmar, la aplicación adquiere una transacción `IMMEDIATE`;
el servicio vuelve a leer, validar e insertar usando exclusivamente esa transacción.

```mermaid
flowchart TD
    A["Buscar y seleccionar contratista"] --> B["Preparar ingreso"]
    B --> B1["Leer contratista y empresa<br/>calcular acceso, gafete e ingreso activo"]
    B1 --> C["Vista previa lógica"]
    C --> D["Elegir medio de ingreso<br/>y gafete cuando corresponda"]
    D --> E["Confirmar registrar_entrada"]
    E --> TX["BEGIN IMMEDIATE"]
    TX --> F["Volver a leer el contratista"]

    F --> G{"¿tiene_acceso?"}
    G -->|No| DEN1["Denegar: SinAcceso"]
    G -->|Sí| H{"¿Requiere PRAIND?"}
    H -->|No| OK["Acceso permitido"]
    H -->|Sí| I{"¿Tiene fecha y no está vencida?"}
    I -->|No| DEN2["Denegar: PraindVencido"]
    I -->|Sí| J{"¿Vence dentro de 30 días?"}
    J -->|Sí| WARN["Permitido con advertencia"]
    J -->|No| OK

    OK --> K{"¿Ya tiene ingreso activo?"}
    WARN --> K
    K -->|Sí| ERR1["Error: IngresoActivo"]
    K -->|No| L{"¿Requiere gafete?"}

    L -->|No| N["Guardar gafete como NULL"]
    L -->|Sí| M{"¿Se indicó y está libre?"}
    M -->|No indicado| ERR2["Error: GafeteRequerido"]
    M -->|Ocupado| ERR3["Error: GafeteOcupado"]
    M -->|Sí| O["Asignar gafete"]

    N --> P["Crear registro activo"]
    O --> P
    P --> P1["Copiar empresa y tipo del contratista<br/>guardar medio, fecha y usuario de entrada"]
    P1 --> Q[("registro_ingresos<br/>salida = NULL")]
    Q --> COMMIT["COMMIT"]
    COMMIT --> R["Recargar Ingresos activos"]

    DEN1 --> RB["ROLLBACK<br/>sin movimiento"]
    DEN2 --> RB
    ERR1 --> RB
    ERR2 --> RB
    ERR3 --> RB
```

El bloqueo de escritura sólo existe durante la confirmación final; nunca permanece
abierto mientras el operador busca, revisa o completa el formulario. Si otra escritura
termina primero, la transacción espera y después vuelve a leer el estado ya actualizado.

Reglas para PRAIND y gafete:

| Condición del contratista | Requiere PRAIND | Requiere gafete |
|---|---:|---:|
| Tipo `PRAIND` | Sí | Sí |
| Tipo `IN_HOUSE` | Sí | No |
| Tipo `POR_CORREO` | No | Sí |
| Tipo `SWAT` | No | No |
| Personal de ruta, sin importar el tipo | Sí | No |

Si un contratista no requiere gafete, cualquier número recibido se descarta y se guarda
`NULL`. La base refuerza que solo exista un ingreso activo por contratista y un único uso
activo de cada gafete.

## 4. Registro de salida e historial

```mermaid
flowchart TD
    A["Consultar todos los ingresos activos<br/>sin límite ni paginación"] --> T["Mostrar total real de personas dentro"]
    T --> B{"Seleccionar de la lista<br/>o buscar por gafete"}
    B -->|Lista| C["Cargar registro por ID"]
    B -->|Gafete exacto| C2["Consultar y devolver<br/>el registro activo completo"]
    C2 --> D
    C --> D{"¿Existe y sigue sin salida?"}
    D -->|No| E["Error: RegistroNoActivo"]
    D -->|Sí| F{"¿salida >= entrada?"}
    F -->|No| G["Error: SalidaAnteriorAIngreso"]
    F -->|Sí| H["UPDATE condicionado a<br/>fecha_hora_salida IS NULL"]
    H --> I["Guardar fecha y usuario de salida"]
    I --> J["Registro cerrado<br/>contratista y gafete quedan libres"]
    J --> K["Historial"]
    K --> L["Filtrar por rango, persona, empresa,<br/>tipo, gafete y estado"]
    L --> M["Contar total y devolver página ordenada"]
```

El intervalo del historial es `[desde, hasta)`: incluye `desde`, excluye `hasta` y exige
que `desde < hasta`.

La lista operativa de ingresos activos no se pagina ni tiene un tope silencioso. Un
filtro puede reducir las filas visibles, pero conserva el total real de personas dentro.
La búsqueda exacta por gafete consulta el registro completo directamente y no depende de
que esté presente en las filas filtradas.

## 5. Administración de datos

```mermaid
flowchart LR
    A["Acción de administración"] --> B{"Tipo"}

    B -->|Empresa| E["Normalizar nombre<br/>exigir no vacío y único"]
    B -->|Contratista| C["Normalizar cédula y nombre<br/>validar empresa existente<br/>fecha PRAIND cuando aplica<br/>cédula única"]
    B -->|Usuario| U["Normalizar identidad y exigir cédula única<br/>al crear o cambiar clave: mínimo 8 y Argon2"]

    E --> DB[("SQLite")]
    C --> DB
    U --> R{"¿La transición elimina<br/>el último ROOT activo?"}
    R -->|Sí| X["Rechazar: UltimoRootActivo"]
    R -->|No| DB

    DB --> Q["Recargar la consulta correspondiente"]
```

Los usuarios se activan o desactivan; no se eliminan, porque sus identificadores forman
parte de la auditoría de entradas y salidas.

## 6. Relaciones persistidas

```mermaid
erDiagram
    EMPRESAS ||--o{ CONTRATISTAS : agrupa
    EMPRESAS ||--o{ REGISTRO_INGRESOS : empresa_del_movimiento
    CONTRATISTAS ||--o{ REGISTRO_INGRESOS : genera
    USUARIOS ||--o{ REGISTRO_INGRESOS : registra_entrada
    USUARIOS o|--o{ REGISTRO_INGRESOS : registra_salida

    EMPRESAS {
        integer id PK
        string nombre UK
    }

    CONTRATISTAS {
        integer id PK
        string cedula UK
        string nombre
        integer empresa_id FK
        string tipo_ingreso
        date fecha_vencimiento_praind
        boolean es_personal_ruta
        boolean tiene_acceso
    }

    USUARIOS {
        integer id PK
        string cedula UK
        string nombre
        string password_hash
        string rol
        boolean activo
    }

    REGISTRO_INGRESOS {
        integer id PK
        integer contratista_id FK
        integer empresa_id FK
        datetime fecha_hora_ingreso
        string medio_ingreso
        string tipo_ingreso
        integer gafete_numero
        integer usuario_ingreso_id FK
        datetime fecha_hora_salida
        integer usuario_salida_id FK
    }
```

`registro_ingresos` copia `empresa_id` y `tipo_ingreso` al momento de la entrada. Así, el
movimiento conserva esos valores aunque después cambie el contratista.

## 7. Lecturas y búsqueda

Las lecturas usan consultas especializadas y modelos sin campos sensibles:

```mermaid
flowchart LR
    A["Texto de búsqueda"] --> B{"Longitud útil"}
    B -->|Vacío| C["Listado sin filtro"]
    B -->|1 o 2 caracteres| D["LIKE"]
    B -->|3 o más caracteres| E["SQLite FTS5 trigram"]
    C --> F["JOIN, filtros, límite y offset"]
    D --> F
    E --> F
    F --> G["Read model para la tabla"]
```

FTS5 se usa para contratistas, empresas y usuarios; sus tablas se sincronizan mediante
triggers. Las búsquedas de usuarios nunca exponen el hash de contraseña.

## Observaciones de la implementación actual

- `Root`, `Administrador` y `Operador` se persisten y se muestran en la sesión, pero no
  existe todavía una comprobación de rol al abrir o ejecutar los casos de uso de
  administración. Actualmente, cualquier usuario autenticado alcanza esas operaciones.
- La preparación del ingreso es informativa: la autorización definitiva ocurre de nuevo
  al registrar la entrada.
- El resultado `Permitido` o `PermitidoConAdvertencia` no se guarda en el movimiento. En
  la lista de activos se recalcula con los datos actuales del contratista.
- Las fechas representan hora local y se guardan como texto `YYYY-MM-DD HH:MM:SS`; no son
  UTC.

## Archivos que implementan el flujo

- Arranque y fachada: `src/main.rs`, `src/application.rs`.
- Máquina de estados: `src/tui/app.rs` y los archivos `src/tui/*/state.rs`.
- Casos de uso: `src/services/`.
- Reglas puras: `src/domain/acceso.rs`, `src/domain/registro_ingreso.rs`.
- Entidades: `src/models/`.
- Persistencia, consultas y migraciones: `src/database/`.
