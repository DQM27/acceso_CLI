# Plan: telemetría por estudios y registro de errores

Plan del 2026-10-20. **Reemplaza** a `plan-herramienta-diagnostico.md` y a
`plan-telemetria-ajustable.md` (quedan como historia). Parte de **lo que ya
existe** y lo ordena; no se reescribe desde cero.

## Objetivo

Mejorar la app con **datos reales de producción**: los builds finales (release)
de todos los equipos en uso, sin muestreo. Dos piezas:

1. **Registro de errores:** siempre encendido. Lo hace el **SDK de Sentry**, que
   ya está en las dos apps, mandando a nuestro receptor (`receptor-errores`), no
   a sentry.io.
2. **Telemetría por estudios:** los eventos se agrupan por tipo de estudio, y
   cada estudio se **enciende y se apaga con una regla** según lo que se quiera
   verificar, sin sacar un release.

Los datos son del desarrollador: nada va al panel web ni a la base de operación
del cliente.

## 1. Registro de errores: el SDK de Sentry, apuntado al receptor propio

### Lo que ya captura

| Plataforma | Qué captura hoy | Dónde |
|---|---|---|
| Escritorio (Rust) | Pánicos (automático) y errores inesperados marcados a mano (sincronización, apertura del núcleo, arranque) | crate `sentry` 0.49, `desktop/src-tauri/src/lib.rs` |
| Móvil (Kotlin) | Excepciones no atrapadas, ANR y fallos nativos (automático); trazas legibles (R8 no ofusca) | `io.sentry:sentry-android` 8.9.0, `AndroidManifest.xml` |
| Receptor | Limpia datos personales, agrupa por huella, limita el ritmo, guarda 90 días | `supabase/functions-staging/receptor-errores`, tabla `errores_app` |
| Aviso | Correo "Error nuevo en las apps" | Grafana |

### Lo que falla o falta

| Problema | Efecto | Arreglo |
|---|---|---|
| Si el build no recibe el secreto `SENTRY_DSN`, el código cae a un DSN de respaldo de **sentry.io** (`lib.rs`, `build.gradle.kts`). `build-test-mobile.yml` no lo pasa. | Los errores de esos builds se van al Sentry viejo. `errores_app` sólo tiene los 3 de prueba; los 8 pánicos del escritorio del 18‑oct no llegaron. | Quitar el respaldo (sin URL no se envía) y pasar el secreto en todos los workflows. Confirmar que el secreto de los release apunta al receptor. |
| El frontend del escritorio (React en el webview) no tiene SDK. | Los errores de JS sólo se cuentan en la telemetría (`error_js`), sin mensaje ni traza. | Agregar el SDK de navegador de Sentry al frontend, al mismo receptor. |
| En el móvil, los errores inesperados del núcleo (`interno`, base de datos) se atrapan y no se reportan. Sólo `MrzParser` llama a `Sentry.captureMessage`. | Un pánico de Rust en el móvil llega como excepción atrapada y no se registra. | `Sentry.captureException` en `medirNucleo` para los tipos inesperados, igual que hace el escritorio. |
| La telemetría del pánico guarda sólo el lugar, sin mensaje. | No se sabe qué pasó. | Nada que hacer: el detalle viene por el SDK una vez arreglado lo de arriba. |

## 2. Telemetría por estudios

### Qué sirve y qué no

Basado en los datos de staging del 13 al 19‑oct (filas por semana).

| Evento | Plataforma | Filas/sem | Veredicto |
|---|---|---|---|
| `sesion_inicio`, `arranque`, `salida_anterior`, `cierre` | ambas | ~120 | **Sirve.** Cuántos equipos, qué versión, si murió el proceso. Barato. Ajuste: el motivo 16 del móvil es `PACKAGE_UPDATED` (hoy `DESCONOCIDO_16`). |
| `panic`, `error_js`, `error_telemetria` | ambas | ~20 | **Sirve como conteo** (detecta errores aunque falle el SDK). El detalle viene del SDK. |
| `llamada_nucleo` | ambas | 9 700 | **Sirve, es el centro del rendimiento.** Ajustes abajo. |
| `transicion_ingreso` | ambas | 213 | **Sirve mucho:** es la espera que vive el guarda al registrar un ingreso. |
| `pantalla` | ambas | 800 | Sirve: tiempo hasta dibujar cada pantalla. |
| `frames` (móvil), `ui_bloqueos` (escritorio) | — | 1 300 | Sirve cuando se persiguen trabas de la interfaz. |
| `realtime`, `sincronizacion_auto` | ambas | 1 770 | Sirve cuando se estudia la sincronización. |
| `reloj`, `sesion_unidad` | ambas | 2 790 | Sirve sólo para auditar el reloj confiable o la sesión única; se manda en **cada** sincronización. No va siempre. |
| `muestra_sistema`, `muestra_webview` | ambas | 11 360 | **El mayor volumen** (35 % de las filas) y lo menos consultado. Sólo para estudiar memoria, CPU o batería. |
| `memoria_baja` | móvil | 430 | Mayormente ruido del paso a segundo plano. Sólo en el estudio de recursos, y filtrar niveles. |
| `ocr_sesion`, `camara_info` | móvil | 380 | Sirve para calibrar el escáner. `camara_info` repite datos fijos: basta una vez por versión y equipo. |
| `strictmode`, `retencion` | móvil | ~20 | Sólo en el build de diagnóstico (son pesados). Sin cambio. |

### Los grupos

| Grupo | Eventos | Para qué estudio | Por defecto |
|---|---|---|---|
| `base` | `sesion_inicio`, `arranque`, `salida_anterior`, `cierre`, `panic`, `error_js`, `error_telemetria` | Estado de la flota: equipos, versiones, caídas | **Siempre encendido** |
| `rendimiento` | `llamada_nucleo`, `transicion_ingreso`, `pantalla` | ¿Cuánto espera el guarda? ¿Un cambio mejoró? | Por regla |
| `fluidez` | `frames`, `ui_bloqueos` | ¿Se traba la interfaz? | Por regla |
| `sincronizacion` | `realtime`, `sincronizacion_auto`, `sesion_unidad`, `reloj` | ¿Llegan los datos? ¿Latencia entre equipos? ¿Reloj? | Por regla |
| `recursos` | `muestra_sistema`, `muestra_webview`, `memoria_baja` | Memoria, CPU, batería, fugas | Por regla |
| `escaneo` | `ocr_sesion`, `camara_info` | Calibrar OCR y cámara | Por regla |

Un evento nuevo se asigna a un grupo antes de usarse.

### Cómo se encienden (reusando las reglas)

Ya existe en staging: tabla `telemetria_reglas` y función
`reglas_telemetria_vigentes` (`supabase/scripts/telemetria_reglas_staging.sql`).
Una regla dice a quién le toca (equipo, plataforma, prefijo de versión), qué
eventos (`eventos text[]`) y cuándo vence. El escritorio **ya la consulta en sus
release** (al abrir y tras el login, sin sondeo) y filtra por tipo de evento,
con pruebas.

Lo que se agrega:

1. **Grupos en la base, no en las apps:** una tabla `telemetria_grupos (grupo,
   tipo)`, y la función expande los nombres de grupo de `eventos` a tipos. El
   escritorio no cambia: sigue recibiendo una lista de tipos. Mover un evento de
   grupo es un `update`, sin release.
2. **`base` siempre:** una regla permanente (`vence_en` lejano) para todos con
   `eventos = '{base}'`. Es la única regla que no vence.
3. **El móvil lee las reglas** igual que el escritorio (portar `Reglas::combinar`
   y `Reglas::siguiente` con los mismos casos de prueba). Con eso desaparecen
   `TELEMETRIA_EN_PRODUCCION` y `TELEMETRIA_LIGERA`: el release del móvil lleva
   la medición siempre, encendida por regla.
4. **Porcentaje de la flota** en la regla (opcional, después): encender un
   estudio en la mitad de los equipos para comparar un cambio con la otra mitad.

Ejemplos de uso:

```sql
-- Estudio de rendimiento en toda la flota de escritorio 1.8.x por 7 días
insert into public.telemetria_reglas (plataforma, version_desde, eventos, vence_en, nota)
values ('escritorio', 'desktop-1.8.', '{rendimiento}', now() + interval '7 days',
        'efecto del HTTP/2');

-- Recursos y fluidez en un solo teléfono por 24 h
insert into public.telemetria_reglas (dispositivo, eventos, nota)
values ('<uuid>', '{recursos,fluidez}', 'posible fuga de descriptores');

-- Apagar todos los estudios (base sigue)
update public.telemetria_reglas set activa = false
where activa and eventos <> '{base}';
```

### Ajustes del grupo `rendimiento`

Lo que impidió saber si el HTTP/2 mejoró algo:

- El release del móvil no lleva el commit en `version_app` (`1.5.0 (38)`); el
  escritorio sí (`desktop-1.8.15+15c0346`). Agregarlo.
- `p50`/`p90` por ventana de 30 s no se pueden combinar. Agregar cubos fijos de
  histograma (`5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000` ms) a
  `llamada_nucleo`; los p50/p90 se quedan para no romper los tableros.
- Separar el tiempo de red del local y contar cuántas peticiones fueron HTTP/2.

## Grafana

- Una vista de **reglas vigentes** (qué estudios están encendidos, dónde y hasta
  cuándo).
- Una sección por grupo en el tablero de telemetría, y el tablero de errores
  sobre `errores_agrupados`.
- Alertas que siguen: error nuevo y equipo nuevo en vinculación. Esta última
  depende de que `base` esté encendido siempre, lo que deja de depender de cómo se
  compiló el release.

## Fases

Cada fase termina con **el release instalado en los equipos reales**.

| # | Fase | Terminada cuando |
|---|---|---|
| 0 | **Errores al receptor:** quitar el DSN de respaldo a sentry.io; secreto en todos los workflows; confirmar el de los release. | Un error de prueba del release de cada plataforma llega a `errores_app` y llega el correo. |
| 1 | **Grupos en la base:** tabla `telemetria_grupos`, función que expande grupos, regla permanente de `base`. | En el escritorio, una regla `{rendimiento}` manda sólo esos eventos, y sin ella sólo llega `base`. |
| 2 | **Reglas en el móvil** y fuera las variables de compilación. | Lo mismo que la fase 1, en el teléfono, con los mismos casos de prueba que el escritorio. |
| 3 | **Huecos de errores:** SDK en el frontend del escritorio y `captureException` de los errores inesperados del núcleo en el móvil. | Un error de JS y un error interno del núcleo de prueba llegan con traza. |
| 4 | **Ajustes de `rendimiento`:** commit en el móvil, cubos de histograma, red frente a local y HTTP/2. | El tablero compara dos versiones por commit. |
| 5 | **Recortes:** `camara_info` una vez por versión, niveles de `memoria_baja`, motivo 16 con nombre, retención programada, tablero por grupos, contrato reescrito. | `docs/telemetria-contrato.md` describe los grupos y lo que hace el código. |

La fase 0 se puede hacer ya y no depende de las demás.

## Decisiones abiertas

| Decisión | Recomendación |
|---|---|
| Avisar al cliente que sus equipos mandan métricas técnicas y errores (sin datos personales) | Sí, antes de encender estudios en toda la flota. |
| Dónde viven los datos de la flota | Staging por ahora; un proyecto de Supabase propio de observabilidad si el volumen crece. |
| Porcentaje de la flota en las reglas | Después de la fase 2, cuando haga falta el primer experimento. |

## Reglas del proyecto que aplican

Las de `AGENTS.md`: español, nunca el término prohibido para el puesto de
control, commits a nombre de Daniel Quintana sin atribuciones, y verificar antes
de afirmar.
