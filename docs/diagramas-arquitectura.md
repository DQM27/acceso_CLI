# Diagramas de arquitectura y operación

Este documento reúne vistas complementarias de Control de Acceso. La idea no es tener
un único diagrama gigante, sino varias lecturas: qué ve el operador, cómo viajan las
acciones, dónde viven las reglas y qué queda persistido.

## 1. Mapa general

```mermaid
flowchart LR
    OP["Operador / Administrador / Root"]

    subgraph Interfaces["Interfaces de uso"]
        GUI["GUI escritorio<br/>Tauri + React"]
        TUI["TUI clásica<br/>Ratatui"]
        CLI["Comandos técnicos<br/>flags y consola"]
    end

    subgraph Nucleo["Núcleo Rust compartido"]
        APP["AppCore<br/>fachada de casos de uso"]
        AUTH["Autenticación y autorización"]
        DOM["Dominio puro<br/>reglas de acceso"]
        SRV["Servicios<br/>ingresos, catálogos, usuarios, gafetes"]
        HIS["Historial, auditoría y respaldos"]
    end

    subgraph Persistencia["Persistencia local"]
        DB[("SQLite<br/>fuente de verdad")]
        FTS["FTS5<br/>búsqueda trigram"]
        BAK["Respaldos<br/>archivos verificados"]
        PDF["Exportaciones<br/>XLSX / PDF"]
    end

    OP --> GUI
    OP --> TUI
    OP --> CLI
    GUI --> APP
    TUI --> APP
    CLI --> APP
    APP --> AUTH
    APP --> SRV
    SRV --> DOM
    APP --> HIS
    SRV --> DB
    HIS --> DB
    DB --> FTS
    HIS --> BAK
    HIS --> PDF
```

Todas las interfaces llegan al mismo núcleo. La GUI no reimplementa reglas; la TUI y la
GUI comparten `AppCore`, servicios, repositorios, consultas, validación temporal y
mensajes de error de negocio.

## 2. Capas de ejecución

```mermaid
flowchart TB
    subgraph GUI["Entorno gráfico de escritorio"]
        R1["desktop/src/App.tsx<br/>sesión, secciones, modales"]
        R2["pantallas/*.tsx<br/>vistas por dominio"]
        R3["componentes/*.tsx<br/>tabla, modal, sidebar, listas"]
        R4["api/*.ts<br/>única capa que llama invoke"]
    end

    subgraph Tauri["Proceso Tauri"]
        T1["comandos/*.rs<br/>DTO + sesión + mensaje de error"]
        T2["GuiState<br/>AppCore + sesión + candado"]
        T3["plugins<br/>dialog, process, updater, log"]
    end

    subgraph Rust["Núcleo de aplicación"]
        A1["application/*.rs<br/>métodos de AppCore por dominio"]
        A2["services/*.rs<br/>casos de uso"]
        A3["domain/*.rs<br/>reglas puras"]
        A4["database/repositories/*.rs<br/>escrituras"]
        A5["database/queries/*.rs<br/>lecturas"]
    end

    R1 --> R2
    R2 --> R3
    R2 --> R4
    R4 -->|"invoke()"| T1
    T1 --> T2
    T2 --> A1
    A1 --> A2
    A2 --> A3
    A2 --> A4
    A1 --> A5
```

La separación importante es que `pantallas/*.tsx` no invoca Tauri directamente. Cada
dominio tiene su cliente en `api/*.ts`, y cada comando Tauri es una función fina que
obtiene la sesión, llama a `AppCore` y traduce el error al mensaje de usuario.

## 3. Navegación GUI

```mermaid
flowchart TD
    A["App inicia"] --> B{"¿Base requiere ROOT inicial?"}
    B -->|Sí| C["Aviso de configuración inicial<br/>desde consola/TUI"]
    B -->|No| D["Login"]
    D --> E["Shell autenticado"]

    E --> SB["Sidebar<br/>secciones visibles por rol"]
    SB --> ACT["Ingresos activos"]
    SB --> HIS["Historial"]
    SB --> CON["Contratistas"]
    SB --> EMP["Empresas"]
    SB --> AUD["Auditoría<br/>Root / Administrador"]
    SB --> USR["Usuarios<br/>Root / Administrador"]
    SB --> GAF["Gafetes"]

    E --> NI["Modal Nuevo ingreso<br/>Ctrl+Shift+N"]
    E --> SA["Modal Salida<br/>Ctrl+Shift+S"]
    E --> UPD["Aviso de actualización<br/>una vez por sesión"]

    NI --> REF["Señal de refresco"]
    SA --> REF
    REF --> ACT
    E --> OUT["Cerrar sesión"]
    OUT --> D
```

La GUI está organizada como una consola de trabajo: `Shell` mantiene sesión, sidebar,
sección activa, modales globales y refresco de ingresos activos. Cada pantalla conserva
su propio estado de filtros, tablas y formularios.

## 4. Navegación TUI

```mermaid
flowchart TD
    A["Terminal en TUI clásica"] --> C["Configuración inicial<br/>si no existe ROOT"]
    A --> D["Login"]
    C --> D
    D --> E["App TUI<br/>máquina de estados"]

    E --> M["Menú principal"]
    M --> I["Nuevo ingreso"]
    M --> A2["Ingresos activos"]
    M --> H["Historial"]
    M --> C2["Contratistas"]
    M --> E2["Empresas"]
    M --> U["Usuarios<br/>según rol"]
    M --> AU["Auditoría<br/>según rol"]
    M --> G["Gafetes"]
    M --> R["Respaldos<br/>Root"]
    M --> P["Cambiar contraseña"]
    M --> MC["Modo CLI<br/>reinicia el entorno"]

    E --> F2["Salida rápida global<br/>F2"]
    I --> M
    A2 --> M
    H --> M
    C2 --> M
    E2 --> M
    U --> M
    AU --> M
    G --> M
    R --> M
    P --> M
```

La TUI usa estados por pantalla (`src/tui/*/state.rs`) y renderizadores por pantalla
(`src/tui/*/render.rs`). Los estados producen acciones de aplicación; el núcleo ejecuta
la operación y luego el estado recibe el resultado para refrescar la vista.

## 5. Ciclo de una acción

```mermaid
sequenceDiagram
    participant UI as Interfaz
    participant API as API/Estado de pantalla
    participant Core as AppCore
    participant Svc as Servicio
    participant Dom as Dominio
    participant DB as SQLite

    UI->>API: Evento de usuario
    API->>Core: Caso de uso tipado
    Core->>Core: Validar sesión, rol y reloj si aplica
    Core->>Svc: Orquestar operación
    Svc->>Dom: Evaluar reglas puras
    Svc->>DB: Leer o escribir por repositorio/query
    DB-->>Svc: Resultado persistido
    Svc-->>Core: Resultado de negocio
    Core-->>API: DTO o error tipado
    API-->>UI: Estado actualizado, tabla, toast o formulario
```

Las reglas de negocio no viven en el render visual. La interfaz solicita una acción; el
núcleo decide si el actor puede ejecutarla y si el estado actual de SQLite permite
completarla.

## 6. Registro de ingreso

```mermaid
flowchart TD
    A["Buscar contratista"] --> B["Preparar ingreso"]
    B --> C["Leer contratista, empresa, ingreso activo y gafete"]
    C --> D["Mostrar vista previa<br/>permitido, advertencia o bloqueo"]
    D --> E["Elegir medio y gafete si aplica"]
    E --> F["Confirmar"]

    F --> TX["BEGIN IMMEDIATE"]
    TX --> R0["Validar reloj contra último movimiento"]
    R0 --> R1["Validar actor activo"]
    R1 --> R2["Releer contratista, empresa, ingreso activo y gafete"]
    R2 --> P{"¿Reglas de acceso permiten entrar?"}

    P -->|No| X["Rollback<br/>sin movimiento"]
    P -->|Sí| G{"¿Ya tiene ingreso activo?"}
    G -->|Sí| X
    G -->|No| H{"¿Requiere gafete?"}
    H -->|No| I["Guardar movimiento sin número"]
    H -->|Sí| J{"¿Número existe, disponible y libre?"}
    J -->|No| X
    J -->|Sí| K["Asignar número al movimiento"]
    I --> S["Guardar fotografía histórica"]
    K --> S
    S --> DB[("registro_ingresos<br/>salida NULL")]
    DB --> OK["COMMIT<br/>refrescar activos"]
```

La preparación es informativa. La autorización real ocurre de nuevo en la confirmación,
dentro de la transacción de escritura, para evitar decisiones basadas en datos que ya
cambiaron.

## 7. Registro de salida

```mermaid
flowchart TD
    A["Abrir salida desde activos, modal o TUI"] --> B{"Buscar movimiento activo"}
    B -->|Por lista| C["Cargar por ID"]
    B -->|Por número| D["Buscar ingreso activo exacto"]
    C --> E{"¿Sigue activo?"}
    D --> E
    E -->|No| X["Error: registro no activo"]
    E -->|Sí| TX["BEGIN IMMEDIATE"]
    TX --> R0["Validar reloj"]
    R0 --> R1["Validar actor activo"]
    R1 --> F["UPDATE condicionado<br/>salida IS NULL"]
    F --> G["Guardar fecha y operador de salida"]
    G --> H["Movimiento cerrado"]
    H --> I["Contratista y número quedan libres"]
    I --> J["Refrescar activos e historial"]
```

La fila no se mueve a otra tabla. El historial aparece cuando se completa
`fecha_hora_salida` en el mismo movimiento creado al ingresar.

## 8. Gafetes físicos

```mermaid
stateDiagram-v2
    [*] --> Disponible: crear uno o rango
    Disponible --> Perdido: marcar perdido con deudor
    Disponible --> DeBaja: dar de baja
    Perdido --> Disponible: resolver como aparecido
    Perdido --> DeBaja: resolver como pagado
    DeBaja --> [*]
```

```mermaid
flowchart LR
    G["gafetes<br/>estado vigente"] --> I["gafetes_incidentes<br/>historial append-only"]
    C["contratistas"] -->|"deudor mientras está perdido"| G
    U["usuarios"] -->|"quién registró el incidente"| I
    G -->|"número usado en ingreso activo"| R["registro_ingresos.gafete_numero"]
```

El catálogo de gafetes físicos guarda el estado actual del número. Los incidentes
guardan cuándo se reportó o resolvió un problema y quién lo hizo. Los movimientos
históricos conservan el número usado, aunque el catálogo actual cambie después.

## 9. Autorización por rol

```mermaid
flowchart TD
    A["Operación solicitada"] --> B["Verificar usuario activo en SQLite"]
    B --> C{"¿Existe y está activo?"}
    C -->|No| X["Rechazar"]
    C -->|Sí| D{"Rol actual"}

    D -->|Root| R["Puede operación diaria, usuarios, auditoría,<br/>catálogos protegidos y respaldos"]
    D -->|Administrador| AD["Puede operación diaria, usuarios,<br/>auditoría y catálogos protegidos"]
    D -->|Operador| OP["Puede operación diaria, catálogos abiertos,<br/>gafetes y cambio propio de contraseña"]

    AD --> E{"¿Quiere gestionar un Root?"}
    E -->|Sí| X
    E -->|No| OK["Continuar"]
    R --> OK
    OP --> F{"¿Operación restringida?"}
    F -->|Sí| X
    F -->|No| OK
```

El rol guardado en la sesión es una fotografía para la interfaz. Para operaciones
sensibles, `AppCore` consulta el usuario vigente en SQLite; si alguien fue desactivado o
degradado, el cambio aplica en la siguiente operación.

## 10. Datos persistidos

```mermaid
erDiagram
    EMPRESAS ||--o{ CONTRATISTAS : agrupa
    EMPRESAS ||--o{ REGISTRO_INGRESOS : snapshot_empresa
    CONTRATISTAS ||--o{ REGISTRO_INGRESOS : genera
    USUARIOS ||--o{ REGISTRO_INGRESOS : entrada
    USUARIOS o|--o{ REGISTRO_INGRESOS : salida
    USUARIOS ||--o{ AUDITORIA_CAMBIOS : registra
    CONTRATISTAS o|--o{ GAFETES : debe
    GAFETES ||--o{ GAFETES_INCIDENTES : tiene
    USUARIOS ||--o{ GAFETES_INCIDENTES : registra
    CONTRATISTAS o|--o{ GAFETES_INCIDENTES : asociado

    EMPRESAS {
        integer id PK
        string nombre UK
        boolean activo
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
        string contratista_cedula
        string contratista_nombre
        string empresa_nombre
        boolean empresa_activa_snapshot
        datetime fecha_hora_ingreso
        string medio_ingreso
        string tipo_ingreso
        integer gafete_numero
        integer usuario_ingreso_id FK
        string usuario_ingreso_nombre
        date fecha_vencimiento_praind
        boolean es_personal_ruta
        boolean tiene_acceso
        string resultado_acceso
        string motivo_resultado
        integer reglas_version
        datetime fecha_hora_salida
        integer usuario_salida_id FK
        string usuario_salida_nombre
    }

    AUDITORIA_CAMBIOS {
        integer id PK
        datetime fecha_hora
        integer usuario_id FK
        string usuario_nombre
        string entidad
        integer entidad_id
        string entidad_nombre
        string campo
        string valor_anterior
        string valor_nuevo
    }

    GAFETES {
        integer id PK
        integer numero UK
        string estado
        integer contratista_deudor_id FK
    }

    GAFETES_INCIDENTES {
        integer id PK
        integer gafete_id FK
        string tipo
        datetime fecha_hora
        integer usuario_id FK
        integer contratista_id FK
        string motivo_resolucion
    }
```

`registro_ingresos` mezcla referencias vivas con copias históricas. Las referencias
mantienen integridad; las copias explican el movimiento como ocurrió en ese momento,
aunque después cambien nombres, empresas, roles o estados.

## 11. Búsqueda, historial y exportación

```mermaid
flowchart TD
    A["Texto y filtros"] --> B{"Longitud de búsqueda"}
    B -->|0 caracteres| C["Listado normal"]
    B -->|1-2 caracteres| D["LIKE con plegado de texto"]
    B -->|3+ caracteres| E["FTS5 trigram"]
    C --> F["Query por dominio"]
    D --> F
    E --> F
    F --> G{"¿Historial paginado?"}
    G -->|No| H["Filas visibles"]
    G -->|Sí| I["Transacción de lectura<br/>total + página + corte máximo"]
    I --> J["Página estable"]
    F --> K{"¿Exportar?"}
    K -->|XLSX| L["Generar archivo completo<br/>del filtro vigente"]
    K -->|PDF| M["Generar reporte PDF"]
```

El historial usa un corte estable para que los movimientos nuevos no desplacen páginas
ya abiertas. La exportación toma el filtro completo, no sólo la página visible.

## 12. Arranque, base y respaldos

```mermaid
flowchart TD
    A["Iniciar proceso"] --> B["Resolver ruta de base<br/>CONTROL_ACCESO_DB o LOCALAPPDATA"]
    B --> C["Adquirir candado por archivo"]
    C --> D["Abrir SQLite"]
    D --> E["PRAGMA de seguridad y durabilidad"]
    E --> F["Verificar archivo propio e integridad rápida"]
    F --> G{"¿Migración pendiente?"}
    G -->|Sí| H["Crear respaldo pre-migración"]
    H --> I["Migrar en transacción IMMEDIATE"]
    G -->|No| J["Construir AppCore"]
    I --> J
    J --> K{"¿Usuarios vacío?"}
    K -->|Sí| L["Requerir ROOT inicial"]
    K -->|No| M["Login"]
    L --> M
    M --> N["Sesión activa"]
    N --> O["Operación diaria"]
    N --> P["Respaldos manuales<br/>Root"]
    O --> Q["Respaldo automático diario<br/>desde 01:00 CR"]
```

La base local es la fuente de verdad. Las migraciones son secuenciales con
`PRAGMA user_version`, y los respaldos se validan antes de considerarse confiables.

## Archivos guía

- GUI: `desktop/src/App.tsx`, `desktop/src/pantallas/`, `desktop/src/api/`.
- Puente Tauri: `desktop/src-tauri/src/lib.rs`, `desktop/src-tauri/src/comandos/`.
- Núcleo: `src/application/`, `src/services/`, `src/domain/`.
- Persistencia: `src/database/schema.rs`, `src/database/repositories/`, `src/database/queries/`.
- TUI: `src/tui/`.
- Diagrama lógico detallado: `docs/diagrama-logico.md`.
