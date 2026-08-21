# Refactorización de `tui::app` y estrategia de errores

Fecha de inicio: 2026-08-20

Rama de trabajo: `refactor/app-y-errores`

Commit base: `4967437`

## Objetivo

Reducir el tamaño y el acoplamiento de `src/tui/app.rs` sin reescribir la TUI ni
cambiar su comportamiento. El trabajo debe conservar la navegación explícita,
los estados propios de cada pantalla y la cobertura de pruebas existente.

También se normalizará la implementación de errores con `thiserror`, manteniendo
separados los errores técnicos/tipados de los mensajes que se presentan al
operador.

## Diagnóstico inicial de `app.rs`

Antes de comenzar, `app.rs` tenía 2.971 líneas y era el archivo Rust más grande
del repositorio. El siguiente archivo más grande tenía aproximadamente 934
líneas.

| Métrica | Resultado inicial |
| --- | ---: |
| Líneas totales | 2.971 |
| Líneas de pruebas incrustadas | 1.338 (45 %) |
| Pruebas dentro del archivo | 27 |
| Campos de `App` | 23 |
| Variantes de `Vista` | 12 |
| Estados visuales administrados | 13, incluido `SalidaRapidaState` |
| Despachadores `procesar_accion_*` | 12 |
| Apariciones de `Option<&AppCore>` | 25 |
| Canales asincrónicos sondeados | 4 |
| Commits que habían tocado `app.rs` | 40 de 77 |

### Veredicto

`App` no es un objeto Dios de dominio: no contiene SQL, reglas centrales ni el
render detallado de cada pantalla. Sí es un coordinador excesivamente grande y
`app.rs` es un hotspot con tendencias de *God file*.

El problema principal es la amplificación de cambios. Agregar o modificar una
pantalla puede exigir tocar en un mismo archivo:

- imports y campos de estado;
- `Vista` y navegación;
- render de la vista activa;
- despacho de teclas y acciones;
- cargas iniciales y recargas cruzadas;
- `tick` y debounce;
- pruebas de integración de la TUI.

## Responsabilidades encontradas

1. Definición de `Vista`, `SalidaApp` y `App`.
2. Construcción y preferencias visuales.
3. Bucle de terminal, render, eventos y temporización.
4. Navegación global, F2, cambio de tema y Ctrl+1..Ctrl+9.
5. Adaptación de acciones de pantalla hacia `AppCore`.
6. Coordinación de sesión y recargas entre pantallas.
7. Trabajos de Argon2 para login, ROOT inicial y contraseñas.
8. Traducción de errores tipados a mensajes para el operador.
9. Pruebas unitarias e integraciones de la aplicación.

## Riesgos que deben conservarse bajo prueba

- Una vista autenticada no debe quedar activa sin sesión.
- Una salida mediante F2 debe refrescar Activos, Historial o Nuevo Ingreso según
  la vista que esté debajo del overlay.
- Los trabajos de Argon2 no deben bloquear el bucle normal ni perder una
  escritura validada durante el cierre.
- La restauración sólo debe devolver `SalidaApp::Restaurar`; `App` no debe
  reemplazar directamente la base activa.
- El modo sin `AppCore` debe continuar mostrando el error de arranque sin dejar
  formularios esperando indefinidamente.
- Las preferencias visuales nunca deben interrumpir una operación de acceso.

## Decisiones de diseño

- No introducir un framework de pantallas ni `dyn Trait` durante esta
  refactorización.
- Mantener los `match` exhaustivos sobre `Vista`.
- No convertir los estados persistentes de las pantallas en un único enum: se
  perdería estado al navegar o habría que introducir otra caché.
- Realizar primero movimientos mecánicos y después cambios estructurales.
- Ejecutar `cargo fmt`, toda la suite y Clippy estricto después de cada corte.
- No exponer miembros privados únicamente para mover pruebas a `tests/`.

## Plan y estado

### Fase 1 — Separar pruebas de `app.rs` — completada

Las 27 pruebas se movieron a `src/tui/app/tests.rs` mediante un módulo hijo con
`#[path = "app/tests.rs"]`. Conservan acceso a los detalles privados de `App`.

Resultado: `app.rs` pasó de 2.971 a 1.633 líneas sin modificar lógica.

### Fase 2 — Normalizar errores — completada

Estado inicial:

- `thiserror` sólo llegaba como dependencia transitiva de Ratatui;
- existían 16 implementaciones manuales de `Display` y `Error`;
- existían 15 conversiones manuales `From<...Error>`;
- `app.rs` contenía cinco funciones `mensaje_*` para la presentación.

Trabajo realizado:

1. Se declaró `thiserror` como dependencia directa.
2. Los 16 errores propios usan `#[derive(thiserror::Error)]`.
3. Se eliminaron las 16 implementaciones manuales de `Display`/`Error` y 14 de
   las 15 conversiones `From` manuales.
4. Se conservó manualmente `From<SchemaError> for RespaldoError` porque no es un
   envoltorio: traduce distintas fallas de esquema a resultados semánticos de
   validación y restauración.
5. Los mapeadores de presentación se movieron a
   `src/tui/app/error_messages.rs`.
6. Se agregaron pruebas para verificar que la TUI no exponga detalles técnicos,
   que las denegaciones conserven mensajes accionables y que los errores de
   respaldo mantengan contexto y `source`.

Resultado adicional: tras separar pruebas y mensajes, `app.rs` quedó en 1.556
líneas.

### Fase 3 — Extraer trabajos asincrónicos — pendiente

Mover receptores, datos pendientes y coordinación de Argon2 a
`src/tui/app/auth_jobs.rs`. Primero se moverán bloques `impl App` sin cambiar el
modelo; sólo después se evaluará encapsularlos en un `AuthJobs`.

### Fase 4 — Agrupar despachadores — pendiente

Separar por área funcional para evitar tanto el archivo monolítico como un
archivo diminuto por pantalla:

```text
src/tui/app/actions/accesos.rs
src/tui/app/actions/catalogos.rs
src/tui/app/actions/admin.rs
```

### Fase 5 — Separar navegación y runtime — pendiente

Destino tentativo:

```text
src/tui/app/navigation.rs
src/tui/app/runtime.rs
```

También se evaluará sustituir `Option<&AppCore>` por un modo de ejecución
explícito que diferencie operación normal y arranque degradado.

## Estrategia de errores objetivo

La cadena debe conservar tipos hasta el límite de presentación:

```text
Error de infraestructura
    -> error de servicio/aplicación tipado
    -> mapeador de presentación TUI
    -> mensaje seguro para el operador
```

`thiserror` se utilizará para expresar `Display`, `source` y `From` sin
boilerplate. No se utilizará para convertir indiscriminadamente errores técnicos
en texto visible. Los errores inesperados de SQLite, E/S o XLSX deben conservar
su fuente para diagnóstico, mientras la TUI decide cuánto detalle mostrar.

## Criterios de finalización

- No cambia el comportamiento observable de navegación ni operaciones.
- No se debilita la tipificación de errores.
- No se muestran detalles internos nuevos al operador.
- Toda la suite pasa con todas las características.
- Clippy pasa con advertencias tratadas como errores.
- `app.rs` queda como composición y coordinación de alto nivel, no como lugar
  obligatorio para implementar cada caso de uso.
