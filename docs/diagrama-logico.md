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
    Q --> X["Administrar usuarios<br/>(oculto para Operador)"]
    Q --> Y2["Configuración → Respaldos<br/>(oculto para Operador)"]

    R -->|Volver| Q
    S -->|Volver| Q
    T -->|Volver| Q
    V -->|Volver| Q
    W -->|Volver| Q
    X -->|Volver| Q
    Y2 -->|Volver| Q
    Q -->|Cerrar sesión| L
    Q -->|Salir| Z["Restaurar terminal y finalizar"]

    F2["Salida rápida (F2)<br/>global, con sesión iniciada,<br/>desde cualquier pantalla"]
```

El inicio ROOT es atómico: dos instancias no pueden crear simultáneamente dos usuarios
iniciales. La autenticación nunca devuelve el `password_hash` dentro de la sesión.

**Salida rápida (F2):** overlay global que registra la salida de un ingreso activo por
gafete o por nombre/cédula sin navegar hasta Ingresos activos, alcanzable con sesión
iniciada desde cualquier pantalla (`src/tui/salida_rapida/`).

**Configuración → Respaldos:** entrada 7 del menú (visible sólo para ROOT y
Administrador) con Crear, Listar, Revalidar, Exportar y Restaurar respaldos —
ver [plan de respaldos](plan-respaldos.md).

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
    P --> P1["Tomar fotografía del momento:<br/>identidad, empresa, PRAIND, acceso,<br/>resultado, reglas y operador"]
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
    H --> I["Guardar fecha, ID y nombre<br/>del usuario de salida una sola vez"]
    I --> J["Registro cerrado<br/>contratista y gafete quedan libres"]
    J --> K["Historial"]
    K --> L["Filtrar por rango, persona, empresa,<br/>tipo, gafete y estado"]
    L --> M["Abrir lectura transaccional<br/>fijar corte máximo de ID"]
    M --> N["Contar total y devolver página<br/>sobre el mismo corte"]
```

El intervalo del historial es `[desde, hasta)`: incluye `desde`, excluye `hasta` y exige
que `desde < hasta`.

La lista operativa de ingresos activos no se pagina ni tiene un tope silencioso. Un
filtro puede reducir las filas visibles, pero conserva el total real de personas dentro.
La búsqueda exacta por gafete consulta el registro completo directamente y no depende de
que esté presente en las filas filtradas.

El total y las filas del historial se consultan dentro de una misma transacción de
lectura. La primera página fija el ID máximo visible y las páginas siguientes conservan
ese corte; por eso un ingreso nuevo no desplaza filas, no produce duplicados y sólo
aparece al iniciar una navegación nueva.

La política temporal es única para toda la aplicación:

```text
reloj del sistema → instante UTC → reglas de calendario en America/Costa_Rica
                                → SQLite: YYYY-MM-DDTHH:MM:SSZ
                                → TUI: fecha y hora de Costa Rica
```

`AppCore` toma la hora de entradas y salidas; la TUI ya no entrega fechas obtenidas de la
zona configurada en Windows. Los filtros escritos como fechas de Costa Rica se convierten
a límites UTC antes de consultar. La migración convierte los movimientos anteriores,
que representaban hora local costarricense, y SQLite rechaza nuevos formatos sin zona.
Antes de cada movimiento se compara el reloj con el último instante persistido; un
retroceso detiene la operación y solicita corregir la fecha y hora del equipo.

No existe una segunda tabla de historial ni se mueve físicamente el registro. El mismo
movimiento nace activo con `fecha_hora_salida = NULL`; al registrar la salida se completa
esa misma fila y pasa a verse como cerrado. Desde el ingreso se guardan copias de los
datos que explican quién entró y bajo qué condiciones:

- cédula y nombre del contratista;
- nombre de la empresa y tipo de ingreso;
- PRAIND, condición de personal de ruta y estado de acceso evaluados;
- resultado (`PERMITIDO` o `PERMITIDO_CON_ADVERTENCIA`), motivo y versión de reglas;
- nombre de los operadores de entrada y salida.

Los identificadores originales se conservan como referencias, pero el historial se
muestra y se busca usando esas copias. Por eso renombrar al contratista, cambiarlo de
empresa o renombrar a un operador no reescribe el pasado. SQLite impide modificar los
datos de entrada, registrar dos salidas o eliminar movimientos. Los movimientos creados
antes de esta mejora se copian con los datos disponibles al migrar y se identifican como
`MIGRADO / DATOS_RECONSTRUIDOS`; no se presentan como una fotografía exacta que nunca
existió.

## 5. Administración de datos

```mermaid
flowchart LR
    A["Acción de administración"] --> B{"Tipo"}

    B -->|Empresa| E["Normalizar nombre<br/>exigir no vacío y único"]
    B -->|Contratista nuevo| C["Normalizar cédula y nombre<br/>validar empresa existente<br/>fecha PRAIND cuando aplica<br/>cédula única"]
    B -->|Editar contratista| CE["Cédula inmutable<br/>actualizar nombre, empresa,<br/>tipo, PRAIND, ruta o acceso"]
    B -->|Usuario| U["Normalizar identidad y exigir cédula única<br/>al crear o cambiar clave: mínimo 8 y Argon2"]

    E --> DB[("SQLite")]
    C --> DB
    CE --> DB
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
        string contratista_cedula_snapshot
        string contratista_nombre_snapshot
        string empresa_nombre_snapshot
        datetime fecha_hora_ingreso
        string medio_ingreso
        string tipo_ingreso
        integer gafete_numero
        integer usuario_ingreso_id FK
        string usuario_ingreso_nombre_snapshot
        date fecha_vencimiento_praind_snapshot
        boolean es_personal_ruta_snapshot
        boolean tiene_acceso_snapshot
        string resultado_acceso
        string motivo_resultado
        integer reglas_version
        datetime fecha_hora_salida
        integer usuario_salida_id FK
        string usuario_salida_nombre_snapshot
    }
```

Las columnas marcadas conceptualmente como `snapshot` son copias históricas; en SQLite
sus nombres no llevan ese sufijo. Los IDs mantienen integridad referencial y las copias
mantienen el significado histórico del evento.

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

- `Root`, `Administrador` y `Operador` se persisten y se muestran en la sesión.
  `Usuarios` y `Configuración` ya están ocultas del menú (y de sus atajos de teclado) para
  `Operador` (`OpcionMenu::visibles_para`, `src/tui/menu_principal/state.rs`). Fuera de esa
  visibilidad de menú, `Root` y `Administrador` siguen siendo funcionalmente idénticos —
  no hay ninguna otra operación que distinga entre ambos salvo la protección de no poder
  desactivar/degradar al último `Root` activo.
- La preparación del ingreso es informativa: la autorización definitiva ocurre de nuevo
  al registrar la entrada.
- El historial conserva el resultado evaluado al ingresar. La lista operativa de activos
  también recalcula el acceso actual para advertir si una autorización fue revocada
  mientras la persona todavía está dentro, sin modificar la fotografía histórica.
- Las fechas se guardan como instantes UTC canónicos y se presentan en
  `America/Costa_Rica`; no dependen de la zona horaria configurada en el equipo.

## Archivos que implementan el flujo

- Arranque y fachada: `src/main.rs`, `src/application.rs`.
- Máquina de estados: `src/tui/app.rs` y los archivos `src/tui/*/state.rs`.
- Casos de uso: `src/services/`.
- Reglas puras: `src/domain/acceso.rs`, `src/domain/registro_ingreso.rs`.
- Política temporal y reloj: `src/tiempo.rs`.
- Entidades: `src/models/`.
- Persistencia, consultas y migraciones: `src/database/`.
