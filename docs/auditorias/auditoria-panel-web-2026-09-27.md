# Auditoría del panel web administrativo (`web/`) (2026-09-27)

Alcance: el **panel administrativo** (`web/`, Cloudflare + Supabase), **no**
`web-visitas/`. Documento pensado para que un agente lo aplique por fases.
El usuario final **no es el desarrollador**: es una persona administrativa,
no técnica, que usa el panel para supervisar varias unidades operativas.
Cada decisión de este documento se juzga con esa persona en mente.

## Reglas para el agente

- Leer `AGENTS.md` (español; nunca el término prohibido para el puesto de
  control: usar "puesto de control", "portería" o "punto de acceso";
  commit documentado + push por cada cambio exitoso).
- **Nunca** tocar Supabase de producción (`control-acceso-nube`). Toda
  migración nueva va primero al repo (`supabase/migrations/`) y se prueba
  en staging (`control-acceso-staging`); producción sólo la aplica el
  dueño del proyecto.
- No agregar administradores del panel desde el panel (decisión de
  seguridad vigente, ver comentario en `web/src/App.tsx`).
- Mantener las Edge Functions actuales y sus contratos salvo que una fase
  diga lo contrario.

## Veredicto

**Reconstruir la capa de interfaz y de datos; conservar la
infraestructura.** No es un rehacer desde cero.

| Capa | Estado | Decisión |
|---|---|---|
| Stack (React 19 + Vite + TS + supabase-js + Edge Functions + Cloudflare) | Correcto para esta escala | **Conservar** |
| Auth (Google OAuth + `administradores_panel` + RLS `es_admin_global()`) | Bien pensado y seguro | **Conservar** |
| Validación de contratos (zod en `api/*`, `invocar` con guardas) | Bueno | **Conservar** |
| Tokens de diseño (`design/brisas.json` → CSS) | Bueno | **Conservar**, aplicarlo de verdad |
| Componentes base (`Tabla`, `Modal`, `ConfirmacionSensible`, `SelectorRangoFecha`) | Útiles | **Conservar** y ampliar |
| Navegación / arquitectura de información | Insuficiente para el usuario final | **Rehacer** |
| Pantallas (4) | Funcionales pero mínimas, inconsistentes | **Rehacer** |
| Acceso a datos (fetch ad hoc + recarga completa) | No escala | **Rehacer** |
| Cobertura funcional | ~40 % de lo que un panel de este tipo necesita | **Completar** |

Estimado: se conserva ~30 % del código (infra, auth, api, componentes
base), se reescribe ~70 % (shell y pantallas) y se agregan 5-6 secciones
nuevas más algunas vistas/funciones en Supabase.

## Diagnóstico

### D1. Arquitectura de información: faltan las preguntas que se hace un administrador

Hoy hay 4 secciones: Historial, Contratistas, Usuarios, Dispositivos
(`web/src/App.tsx`, `SECCIONES`). La persona administrativa llega con
preguntas como:

- "¿Quién está adentro **ahora** en cada unidad?" → **no existe**.
- "¿Hay algún puesto de control sin conexión?" → escondido en una columna
  de Dispositivos.
- "¿Qué contratistas tienen el PRAIND vencido o por vencer?" → sólo
  ordenando una columna a mano.
- "¿Qué pasó con proveedores, visitas, rutas o gafetes provisionales?" →
  **no existe**: Historial sólo muestra la tabla `ingresos`
  (contratistas), aunque la nube ya tiene `ingresos_proveedor`,
  `movimientos_visita`, `salidas_ruta`/`viajes_ruta` y
  `prestamos_gafete_provisional`.
- "¿Quién dio de baja a esta persona y por qué?" → **no queda registro**
  de las acciones hechas desde el panel.
- "Necesito un informe del mes por unidad/empresa" → **no existe**
  (era el punto 5 del alcance original en
  `docs/planes-implementados/plan-panel-administrativo-web.md`).
- Unidades operativas (sitios): sólo se pueden **crear**, y desde un
  botón dentro del modal de "Nuevo dispositivo". No hay dónde verlas,
  renombrarlas ni desactivarlas.

### D2. Acciones críticas sin protección ni rastro

- **Dar de baja a un contratista es un solo clic** en un interruptor de
  la grilla (`pantallas/Contratistas.tsx`, columna "Activo" con
  `InterruptorCelda`), sin confirmación, sin motivo y sin registro. Lo
  mismo con usuarios (`pantallas/Usuarios.tsx`). Un clic accidental
  niega el acceso a una persona en **todas** las unidades.
- Dispositivos sí usa `ConfirmacionSensible` para revocar/eliminar: el
  criterio existe, pero no se aplica de forma pareja.

### D3. Datos: se recarga todo, siempre

- Historial trae hasta **20.000 filas** del rango (por defecto 6 meses)
  (`api/historial.ts`, `LIMITE_HISTORIAL`) y filtra en el navegador.
- `useAutoRefresh` (`componentes/useAutoRefresh.ts`) vuelve a pedir
  **la lista completa** ante **cualquier** cambio de la tabla por Realtime
  y además cada 30 s en Historial. Con varias unidades registrando
  ingresos, el panel abierto baja la misma tabla grande una y otra vez.
- Contratistas: hasta 10.000 filas, misma estrategia.
- Las secciones visitadas quedan montadas y cada una mantiene su canal y
  su intervalo (`App.tsx`, comentario "Cada sección visitada se queda
  MONTADA").
- Cada pantalla repite a mano `cargando`/`error`/`recargar` con
  `Promise.resolve().then(...)` para esquivar un lint: no hay una capa
  común de datos (caché, deduplicación, invalidación).

### D4. Consistencia visual y de código

- 92 objetos `style={{...}}` en línea contra 5 usos de clases de Tailwind,
  aunque Tailwind está instalado (`@tailwindcss/vite`) y hay tokens en
  `design/`. Tres fuentes de estilo a la vez: `index.css` (543 l.),
  `controles.css`, `diseno.css` + estilos en línea.
- Las pantallas **no tienen título**: no hay encabezado con el nombre de
  la sección, una descripción corta ni la acción principal en un lugar
  fijo. Tampoco cambia `document.title`.
- El menú lateral se colapsa con **doble clic** (`componentes/Sidebar.tsx`,
  `onDoubleClick`): nadie lo va a descubrir.
- El menú de usuario y el selector de tema están en una barra de estado
  **abajo** (`barra-estado`), no donde un usuario los busca (arriba a la
  derecha).
- Los filtros no viven en la URL: recargar la página o compartir un
  enlace pierde lo filtrado; el botón "atrás" no deshace filtros.

### D5. Lenguaje

- Mezcla de voseo e imperativos rioplatenses en textos visibles
  ("Acotá las fechas…" en `Historial.tsx`; "probá iniciar sesión de
  nuevo" en `AuthContexto.tsx`). Definir **un** tono (recomendado:
  *usted*, neutro y formal) y aplicarlo en todo.
- Jerga técnica visible: "truncado", guiones dobles `--` en mensajes,
  "dispositivo provisionado", "secreto". Reescribir para una persona no
  técnica.
- El doc-comment de `ResultadoHistorial` (`api/historial.ts`) dice que
  exportar trae el rango completo; en realidad exporta sólo lo cargado
  (`filasParaExportar` en `Historial.tsx`), como sí dice el aviso en
  pantalla. Corregir el comentario (o el comportamiento, ver F2).

### D6. Pruebas

4 pruebas e2e (`web/e2e/panel.spec.ts`). Los flujos críticos (dar de baja,
provisionar dispositivo, exportar) no tienen e2e.

## Principios de UX/UI para el rediseño

1. **Cada pantalla responde una pregunta.** Encabezado fijo: título,
   una línea de descripción, acción principal a la derecha.
2. **Primero lo urgente.** El inicio muestra estado y alertas, no una tabla.
3. **Nada destructivo en un clic.** Bajas, revocaciones y eliminaciones:
   menú de acciones de la fila → diálogo con consecuencia explicada en
   lenguaje llano + motivo obligatorio para bajas. Acciones reversibles
   menores: aviso con "Deshacer".
4. **Estado con texto, no sólo color** (chips "Activo", "Suspendido",
   "Sin conexión hace 3 h"). Accesible para daltonismo.
5. **Estados vacíos que enseñan** ("Todavía no hay dispositivos. Cree el
   primero con *Nuevo dispositivo*").
6. **Ficha antes que grilla** para personas y dispositivos: clic en una
   fila abre un panel lateral con todos los datos y sus últimos
   movimientos.
7. **Filtros en la URL** (`?desde=…&unidad=…`), con botón "Limpiar
   filtros" y filtros comunes guardables.
8. **Un solo tono** (usted), fechas y números `es-CR`, sin jerga.
9. **Carga percibida**: esqueletos en vez de "Cargando…"; los refrescos
   automáticos nunca mueven la fila que la persona está mirando.
10. **Accesibilidad**: foco visible, navegación con teclado, `aria-*` en
    diálogos, contraste AA con los tokens actuales (claro y oscuro).

## Arquitectura de información propuesta

| Sección | Qué responde | Contenido |
|---|---|---|
| **Inicio** (nuevo) | ¿Cómo está todo ahora? | Por unidad: personas adentro (contratistas, proveedores, visitas, rutas), gafetes prestados, puestos de control sin conexión, PRAIND vencidos / por vencer (30 días), conflictos (misma persona adentro en dos unidades). Cada tarjeta lleva a su lista filtrada. |
| **Movimientos** (rehecho) | ¿Qué pasó? | Historial unificado con filtro por tipo (Contratistas, Proveedores, Visitas, Rutas, Gafetes provisionales), unidad, fecha, empresa, texto. Paginado en el servidor. Detalle en panel lateral. Exportar el rango **completo** a Excel/PDF. |
| **Contratistas** (rehecho) | ¿Quién puede entrar? | Lista + ficha (datos, empresa, PRAIND, estado, últimos movimientos). Alta y edición. Baja/reactivación con motivo y confirmación. Filtros rápidos: activos, dados de baja, PRAIND vencido, por vencer. |
| **Empresas** (nuevo) | ¿Con qué empresas trabajamos? | Empresas de contratistas y empresas proveedoras; cantidad de personas; detectar y **fusionar** duplicados. |
| **Rutas** (nuevo, sólo lectura al inicio) | Catálogo de rutas | Rutas, vehículos, encargados. |
| **Usuarios** (rehecho) | ¿Quién opera los puestos de control? | Lista + ficha, alta, restablecer contraseña, baja con motivo. |
| **Unidades y dispositivos** (rehecho) | ¿Dónde y con qué se opera? | Página de unidades (crear, renombrar, desactivar) y, dentro de cada una, sus dispositivos con estado de conexión claro. |
| **Reportes** (nuevo) | Números para informar | Ingresos por unidad/empresa/período, tiempo promedio de permanencia, horas pico, personas únicas; exportable. |
| **Actividad del panel** (nuevo) | ¿Quién cambió qué? | Registro de acciones hechas desde el panel: quién, qué, cuándo, motivo. |

Menú agrupado: *Operación* (Inicio, Movimientos) · *Catálogos*
(Contratistas, Empresas, Rutas) · *Administración* (Usuarios, Unidades y
dispositivos, Actividad del panel) · *Reportes*.

## Cambios técnicos

1. **Capa de datos común**: adoptar `@tanstack/react-query` (una
   dependencia, justificada: caché, deduplicación, reintentos,
   invalidación). Realtime deja de recargar listas completas: sólo
   **invalida** la consulta afectada. Eliminar los
   `Promise.resolve().then(...)` y el estado `cargando/error` repetido.
2. **Paginado y filtros en el servidor** para Movimientos y Contratistas
   (`.range()` + filtros de supabase-js; con AG Grid Community usar el
   *Infinite Row Model* o paginación simple). Quitar los topes de 10.000
   y 20.000 filas como mecanismo principal.
3. **Vistas y funciones en Supabase** (migraciones en el repo, probadas en
   staging), todas con `security_invoker = true` para respetar RLS:
   - `panel_presentes_ahora` (unión de abiertos por tipo y unidad).
   - `panel_movimientos` (unión normalizada de los 5 tipos de movimiento).
   - `panel_alertas` (PRAIND vencidos/por vencer, conflictos, dispositivos
     sin conexión).
   - RPC de reportes (agregados por unidad/empresa/período).
   - Tabla `panel_actividad` + escritura desde las Edge Functions y desde
     triggers de las tablas que el panel modifica (quién, acción, entidad,
     motivo, antes/después). Sólo lectura para `es_admin_global()`.
   - Columna/parámetro de **motivo** para bajas de contratistas y
     usuarios (vía RPC o Edge Function, no `update` directo desde el
     navegador).
4. **Rutas reales** con `react-router` y parámetros en la URL; mantener la
   carga diferida por sección. Evaluar si mantener montadas las secciones
   visitadas sigue haciendo falta con la caché de React Query (lo más
   probable es que no).
5. **Un solo sistema de estilos**: Tailwind v4 con los tokens de
   `design/brisas.json` como tema (`@theme`), componentes con clases, cero
   `style={{}}` salvo valores dinámicos. `index.css` queda sólo para
   AG Grid y lo global.
6. **Componentes de interfaz** nuevos y reutilizables: `EncabezadoPagina`,
   `PanelLateral` (ficha), `MenuAccionesFila`, `ChipEstado`,
   `EstadoVacio`, `Esqueleto`, `DialogoBaja` (motivo obligatorio),
   `BarraFiltros` (sincronizada con la URL).
7. **Pruebas**: e2e por flujo crítico (iniciar sesión simulada, dar de
   baja con motivo, provisionar dispositivo, filtrar y exportar
   movimientos, ver Inicio). Unitarias para mapeos de datos y textos.

## Fases (cada una se puede entregar sola)

| Fase | Contenido | Criterio de terminado |
|---|---|---|
| **F0 Fundamentos** | Shell nuevo (menú agrupado con botón visible para colapsar, encabezado con usuario y tema arriba a la derecha), `EncabezadoPagina`, React Query, Tailwind + tokens, rutas con filtros en URL, guía de tono (usted) aplicada a los textos existentes. | Las 4 pantallas actuales funcionan igual dentro del shell nuevo; sin `style={{}}` estáticos; sin voseo; `document.title` por sección. |
| **F1 Seguridad de acciones** | `MenuAccionesFila` + `DialogoBaja` con motivo en Contratistas y Usuarios; tabla `panel_actividad` y su registro; página **Actividad del panel**. | Ninguna baja en un clic; cada acción del panel queda registrada con motivo. |
| **F2 Movimientos** | Vista `panel_movimientos`, pantalla unificada paginada en servidor, detalle lateral, exportación del rango completo. | Se ven los 5 tipos de movimiento; el panel no baja más filas que las de la página visible; Realtime sólo invalida. |
| **F3 Inicio** | Vistas `panel_presentes_ahora` y `panel_alertas`; tarjetas por unidad con enlaces a listas filtradas. | Una persona no técnica puede responder "¿quién está adentro y hay algo mal?" en menos de 10 s. |
| **F4 Catálogos** | Ficha + alta/edición de contratistas; página Empresas con fusión de duplicados; Rutas en sólo lectura; Unidades y dispositivos como página propia (renombrar/desactivar unidad). | CRUD completo de contratistas; unidades gestionables sin pasar por el modal de dispositivos. |
| **F5 Reportes** | RPC de agregados + página con filtros y exportación. | Informe mensual por unidad y empresa exportable a Excel/PDF. |

Orden recomendado: F0 → F1 → F2 → F3 → F4 → F5. F1 va antes que las
funciones nuevas porque hoy hay un riesgo real (bajas accidentales sin
rastro).

## Verificación (antes de cada push)

```bash
cd web
npm ci
npm run lint
npm run build        # tsc + vite
npm test             # vitest
npm run test:e2e     # playwright (usar el Chromium preinstalado)
npm run design:check # tokens de diseño sincronizados
```

Migraciones: aplicar y probar sólo en staging; comprobar RLS con una
cuenta que **no** esté en `administradores_panel` (debe ver cero filas en
todas las vistas nuevas).
