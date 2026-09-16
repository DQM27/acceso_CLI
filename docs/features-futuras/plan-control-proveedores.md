# Módulo "Control de Proveedores"

> Plan aprobado, sin ejecutar todavía — mismo criterio que otros documentos
> de esta carpeta (`plan-control-rutas.md`,
> `plan-login-visitas-cualquier-dominio.md`).

## Contexto

Hoy la app controla contratistas (personas recurrentes, con catálogo propio
`contratistas`) y visitas (`movimientos_visita`), y recién se agregó rutas
de distribución (`salidas_ruta`). Falta un cuarto caso, estructuralmente
distinto a los tres anteriores: **proveedores**. La empresa que abastece
siempre es la misma (Maika, Dos Pinos, ...), pero **la persona que llega
nunca se repite**, y el vehículo tampoco es fijo. Por eso no aplica ni el
patrón de catálogo de `contratistas` (persona reutilizable) ni el de
`vehiculos_ruta`/`encargados_ruta` (activos fijos) — sólo la empresa amerita
catálogo.

Mobile-first, igual que rutas: el flujo completo (OCR de cédula → empresa →
placa opcional → gafete) se opera desde el teléfono; desktop da paridad
operativa como respaldo y administra el catálogo de empresas proveedoras.
Este plan cubre **sólo "control de proveedores"** — "gafetes provisionales"
es un módulo aparte, documentado en
`plan-gafetes-provisionales-kof.md`.

**Orden de ejecución, igual que rutas: núcleo primero, UI después.**

## Hallazgo clave: el esquema ya anticipó esto a medias

`gafetes.tipo` ya acepta `'PROVEEDOR'` en su CHECK
(`src/database/schema.rs:2465`), con un comentario explícito: *"PROVEEDOR
aceptado a futuro, sin columna de portador propia todavía porque no existe
tabla proveedores"* (línea ~2452). El enum Rust `TipoGafete::Proveedor`
(`src/models/gafete.rs:56`) también existe — pero `PortadorGafete` no tiene
variante `Proveedor(i64)` (línea 93, comentario explícito de por qué), y
tanto el DTO de Tauri (`desktop/src-tauri/src/dto/gafetes.rs:33-36`) como el
tipo TS (`desktop/src/api/gafetes.ts:11`) excluyen "proveedor" a propósito,
a la espera de que existiera la entidad. Ahora existe: hay que completar
esa cadena, no rediseñarla.

**Decisión de diseño importante, distinta de lo que el comentario original
asumía:** el comentario de `gafetes.rs` imaginaba una futura tabla
`proveedores` (catálogo de personas). Pero el usuario fue explícito: **no
hay catálogo de personas, cada colaborador es distinto**. Así que el
portador de un gafete tipo `PROVEEDOR` no puede apuntar a un catálogo de
personas — apunta directo al **registro de ingreso** (la visita puntual),
igual de válido como "portador" que un contratista, sólo que la entidad
referenciada es transaccional, no un catálogo.

## Diseño del núcleo

### Esquema (`src/database/schema.rs`)

1. **`empresas_proveedor`** — catálogo nuevo y **separado** de `empresas`
   (el usuario fue explícito: "son empresas aparte de la de los
   colaboradores"). Mismo shape mínimo que `empresas`: `id`, `nombre TEXT
   NOT NULL UNIQUE`, `activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN
   (0,1))`, `uuid TEXT` con índice único. Separarlo evita, además, que un
   choque de nombre entre una empresa-empleadora-de-contratistas y una
   empresa-proveedora (conceptos distintos) rompa el `UNIQUE(nombre)`
   compartido. Repositorio nuevo `empresa_proveedor_repository.rs` y
   servicio `empresa_proveedor_service.rs`, mismo CRUD mínimo que
   `empresa_service.rs` (crear, buscar_por_nombre, listar, activar/
   desactivar) — no hace falta generalizar/compartir código entre ambos
   servicios, son catálogos pequeños y el proyecto ya tolera esta
   duplicación entre `contratista_service`/`encargado_ruta_repository`.

2. **`registro_ingresos_proveedor`** — el registro operativo, sin catálogo
   de persona (a diferencia de `registro_ingresos`, que sí tiene
   `contratista_id NOT NULL`). Mirroring `movimientos_visita` en espíritu:
   - `cedula TEXT NOT NULL`, `nombre TEXT NOT NULL` — snapshot puro, viene
     del OCR o tecleado, sin FK a ningún catálogo de personas.
   - `empresa_id INTEGER NOT NULL REFERENCES empresas_proveedor(id)` +
     `empresa_nombre TEXT NOT NULL` (snapshot, mismo patrón denormalizado
     que `registro_ingresos.empresa_nombre`).
   - `placa TEXT` — nullable, sin catálogo de vehículos (cada uno es
     distinto). La ausencia de placa ya comunica "llegó a pie"; no hace
     falta un campo `medio_ingreso` aparte.
   - `gafete_numero INTEGER NOT NULL` — a diferencia de contratistas
     (`requiere_gafete` condicional), acá el usuario fue explícito: "muy
     importante, se le asigna un gafete" — siempre obligatorio, sin
     condición.
   - `fecha_hora_ingreso TEXT NOT NULL`, `usuario_ingreso_id INTEGER NOT
     NULL REFERENCES usuarios(id)`, `usuario_ingreso_nombre TEXT NOT NULL`.
   - Trío nullable todo-o-nada para el cierre: `fecha_hora_salida`,
     `usuario_salida_id REFERENCES usuarios(id)`, `usuario_salida_nombre`
     — mismo `CHECK` que `registro_ingresos`/`salidas_ruta`.
   - `CHECK` cronológico (`fecha_hora_salida >= fecha_hora_ingreso`).
   - `uuid TEXT` único, para sync.
   - Índice único parcial `WHERE fecha_hora_salida IS NULL` sobre `cedula`
     — evita dos ingresos abiertos a la vez para la misma cédula (mismo
     criterio que ya usa `movimientos_visita`/`salidas_ruta`).
   - Triggers análogos a `movimientos_visita`: no se borra, apertura
     inmutable, cierre una sola vez, fechas normalizadas a UTC.

3. **`gafetes`** — agregar `proveedor_portador_id INTEGER REFERENCES
   registro_ingresos_proveedor(id) ON DELETE RESTRICT` y extender el
   `CHECK` par-exclusivo existente (línea ~2470) para el tercer caso:
   `(tipo = 'PROVEEDOR' AND contratista_portador_id IS NULL AND
   visita_portador_id IS NULL)` pasa a exigir además `proveedor_portador_id
   IS NOT NULL`, y los otros dos casos pasan a exigir `proveedor_portador_id
   IS NULL`. Mismo criterio para el `CHECK` de `estado = 'PERDIDO'`.

4. **`gafetes_incidentes`** — agregar `proveedor_portador_id` con el mismo
   criterio que ya tiene para `contratista_id`/`visita_portador_id`, por
   consistencia (hoy un gafete de proveedor perdido no tendría dónde
   registrar su portador en un incidente).

5. Si este módulo sincroniza a la nube (como rutas), sumar
   `'ingreso_proveedor'` al `CHECK` de `cola_salida.entidad` (mismo patrón
   que `MIGRACION_30` usó para `movimiento_visita`).

### Dominio y servicios (`src/domain/`, `src/services/`)

- `src/models/gafete.rs`: agregar `PortadorGafete::Proveedor(i64)` (el
  `i64` es el id de `registro_ingresos_proveedor`, no de un catálogo de
  personas — dejar un comentario que lo aclare, dado que el comentario
  actual de la línea 93 asume lo contrario).
- Nuevo `ingreso_proveedor_service.rs`, mismo espíritu que
  `registro_ingreso_service.rs` pero sin la complejidad de PRAIND/
  bloqueos/reglas de contratista: `registrar_ingreso_proveedor` (crea el
  registro + asigna el gafete atómicamente, reusando la validación de
  "gafete no ocupado en el sitio" que ya existe para contratistas),
  `registrar_salida_proveedor`, `listar_proveedores_activos`.
- `src/services/gafete_service.rs`: revisar `dar_de_baja`/`marcar_perdido`/
  `resolver` — hoy reciben repos tipados a contratista/visita; extender
  para el caso proveedor (probablemente ya son genéricos vía el repo de
  `RegistroIngresoRepository`-equivalente, confirmar al implementar).

### Capa UniFFI (`mobile/rust-core/src/lib.rs`)

Sumar a la fachada plana de `Nucleo`: `buscar_empresas_proveedor`,
`crear_empresa_proveedor`, `registrar_ingreso_proveedor`,
`registrar_salida_proveedor`, `listar_proveedores_activos`. El agregador
que unifica "todos los activos" (usado hoy por `ActivosViewModel` para
mostrar contratistas + visitas juntos) necesita sumar esta tercera fuente,
con su propia función de cierre (`registrar_salida_proveedor`), igual que
ya distingue fila Local vs Remota.

## UI mobile (después del núcleo)

Nueva `PantallaProveedores.kt` + `ProveedoresViewModel.kt`, **mismo
esqueleto de tarjetas que `PantallaRutas.kt`** (`PasoEncabezado` con
círculo numerado/check, `completado: Boolean` por paso, botón final que
sólo habilita cuando los pasos obligatorios están completos):

1. **Paso 1 — Persona (obligatorio).** A diferencia de `PasoEncargado` de
   rutas (que busca en un catálogo de encargados), acá **no hay catálogo
   contra qué buscar** — es entrada de datos pura, mismo patrón que los
   campos `cedula`/`nombre` de `PantallaNuevoContratista.kt` (editable a
   mano, autocompletado opcional). Botón de cámara abre
   `PantallaEscanearCedula`, reusando el lector ya genérico — agregar un
   valor nuevo `DOCUMENTO_PROVEEDOR` a `ModoEscaneoDocumento`
   (`EstabilizadorLectura.kt:8`) en vez de reusar `DOCUMENTO_CONTRATISTA`,
   sólo para que el copy en pantalla no diga "contratista" (el criterio de
   aceptación de documento es idéntico, mismo `TipoDocumento`).
2. **Paso 2 — Empresa (obligatorio).** Buscador contra
   `buscar_empresas_proveedor` (`ExposedDropdownMenuBox`, mismo componente
   que ya usa `PasoEncargado`/`PantallaNuevoContratista` para elegir
   empresa) + botón "Crear empresa" al lado si no aparece en la búsqueda,
   que llama `crear_empresa_proveedor` inline (diálogo simple de un solo
   campo). Esto es funcionalidad nueva en mobile — hoy sólo existe alta de
   empresas en desktop (`FormularioEmpresa.tsx`).
3. **Paso 3 — Vehículo (opcional).** Placa únicamente (sin número de
   unidad — no aplica, cada vehículo de proveedor es distinto y no hay
   flota fija). Botón de cámara reutiliza `PantallaEscanearVehiculoRuta`/
   `LectorVehiculoRuta.kt` tal cual, quedándose sólo con la detección de
   placa.
4. **Gafete (obligatorio, no es un paso con check sino parte de la
   confirmación final)** — mismo patrón que
   `PantallaConfirmarIngreso.kt:253-296`: campo de texto numérico +
   escaneo opcional con `PantallaEscanearCedula(modo =
   GAFETE_CONTRATISTA)` (evaluar si conviene un modo de escaneo de gafete
   propio o reusar el existente — el gafete físico de proveedor es un
   objeto distinto, revisar al implementar), validado en vivo contra
   "gafete no ocupado en el sitio" antes de confirmar.

Botón final "Registrar ingreso" llama a `registrarIngresoProveedor`.

Cierre de ciclo: se integra a la lista de "Activos" ya existente
(`PantallaActivos.kt`/`ActivosViewModel.kt`) como una tercera categoría de
fila junto a contratistas y visitas — tap + `DialogoConfirmarSalida` +
`registrarSalidaProveedor`, mismo patrón que ya usa
`confirmarSalida`/`registrarSalida`.

## UI desktop (paridad operativa + administración del catálogo)

- **`EmpresasProveedor.tsx`**: mismo esqueleto que `Empresas.tsx` (Tabla +
  `FormularioEmpresaProveedor` modal + cliente API +
  comando Tauri + servicio/repo ya definidos arriba).
- **`Proveedores.tsx`** (o una pestaña dentro de una pantalla combinada):
  registrar ingreso (formulario con los mismos 3 campos + gafete, sin OCR
  — desktop no tiene cámara para esto, igual que rutas), historial, y
  respaldo operativo si falla el celular.
- **`Activos.tsx`**: sumar la tercera fuente al `listarTodosLosActivos()`
  y a la columna de acción "Salida" (`cerrarFilaActiva`), mismo patrón que
  ya distingue fila local/remota.
- **`gafetes.ts`/`dto/gafetes.rs`**: agregar `"proveedor"` a
  `TipoGafeteEntrada`/`TipoGafete` ahora que sí existe la entidad — esto
  reabre exactamente el comentario que hoy dice "no hay comando de alta
  para esa categoría hasta que exista la entidad `proveedor`"
  (`desktop/src/api/gafetes.ts:8-10`).

## Verificación (una vez implementado)

- `cargo test` en el crate raíz — tests de schema/servicios nuevos, mismo
  estilo que los de `registro_ingresos`/`movimientos_visita`/
  `salidas_ruta`; en particular el `CHECK` par-exclusivo de `gafetes` con
  las tres variantes de portador.
- `cargo test` en `mobile/rust-core` para los métodos nuevos de `Nucleo`.
- Build real de Android vía `workflow_dispatch` sobre `release.yml` (mismo
  mecanismo ya usado en esta sesión) para confirmar compilación de punta a
  punta antes de dar por buena la UI.
- Prueba manual del ciclo completo: alta de empresa nueva desde el
  selector, ingreso con OCR de cédula + placa opcional + gafete, bloqueo
  si el gafete ya está en uso, salida desde la lista de Activos, y que
  contratistas/visitas/rutas sigan funcionando sin regresión.
