# Auditoría de la app de escritorio: rendimiento, organización y UX/UI

Fecha: 2026-11-14. Commit revisado: `c7c4e60` (`main`), versión 1.10.9.
Alcance: sólo `desktop/` (frontend React + adaptador Tauri) y `design/`.
No se tocó código: este documento es el único cambio de la rama.

Sirve para que otra sesión continúe el trabajo. Cada hallazgo dice **qué se
vio**, **dónde está**, **cómo se comprobó**, **qué se propone** y **cómo
verificar el arreglo**. La sección 8 lista las decisiones que sólo puede
tomar el dueño del proyecto: no empezar esas tareas sin su respuesta.

Criterio pedido por el usuario para todas las propuestas: *menos es más* y
navaja de Ockham. Ante dos soluciones, la que quita piezas gana.

## 1. Cómo se hizo la revisión

| Qué | Cómo | Límite |
| --- | --- | --- |
| Estructura y código | Lectura de `desktop/src` (223 archivos, ~28.700 líneas con tests) y de `desktop/src-tauri/src` (~6.100 líneas) | El núcleo (`src/` de la raíz) no se auditó |
| Tamaño del bundle | `npm ci && npm run build` en `desktop/` | Medido en Linux; los tamaños no dependen del sistema |
| Pantallas | Vite en modo desarrollo dentro de Chromium, con `window.__TAURI_INTERNALS__` simulado y datos falsos, a 1440×860, en Tokyo Day y Tokyo Night | No es WebView2 ni Windows; los datos son inventados |
| Tiempo de arranque, IPC, SQLite | **No se midió** | Lo dicho sobre el backend sale de leer el código, no de un perfil |

Lo que sí quedó comprobado con una corrida real: los tamaños del bundle, el
aspecto de las pantallas y el recorte de la columna "Acción" a 1440 px. Lo
demás es lectura de código y se marca como tal.

### Cómo repetir las capturas

El frontend arranca fuera de Tauri si antes de cargar la página se define
`window.__TAURI_INTERNALS__` con un `invoke` falso (Playwright,
`page.addInitScript`). Respuestas mínimas que hacen falta:

| Comando | Respuesta |
| --- | --- |
| `requiere_configuracion_inicial` | `false` |
| `login` | `{ sesion: { id, cedula, nombre, rol }, debe_cambiar_password: false }` |
| `identidad_equipo` | `{ unidad, etiqueta }` |
| `equipo_retirado` | `false` |
| `avisos_permanencia` | `{ en_aviso, criticos, texto_aviso, texto_critico }` |
| `listar_ingresos_activos` | `{ items: IngresoActivoResumen[], total }` |
| `listar_historial`, `listar_auditoria` | `{ items: [], truncado: false }` |
| `buscar_contratistas` | `{ items: ContratistaResumen[], total }` |
| `telemetria_grupos` | `[]` |
| `plugin:app\|version` | `"1.10.9"` |
| cualquier otro `listar_*` / `buscar_*` | `[]` |
| el resto | `null` |

Hacen falta también `transformCallback`, `unregisterCallback`,
`metadata.currentWindow.label = "main"` y
`window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} }`.

Aviso: con `reglas_formulario_contratista` en `null`, abrir el formulario de
contratista rompe la sección ("Cannot read properties of null (reading
'admite_personal_ruta')"). **Es un defecto del simulador, no de la app**:
hay que devolver el objeto real de reglas para capturar ese formulario.

## 2. Cómo está construida

- **Pila**: Tauri 2 + React 19.3 + TypeScript 6 + Vite 8 (rolldown) +
  Tailwind 4 + AG Grid Community 36. Formularios con `react-hook-form` y
  `zod`. Avisos con `sonner`. Menú lateral reordenable con `@dnd-kit`.
- **Backend**: `desktop/src-tauri` es un adaptador fino sobre el crate raíz
  `control_acceso`. Estado en `GuiState` (`estado.rs`): un
  `Mutex<AppCore>` compartido por todos los comandos, más una conexión
  secundaria a SQLite para consultas largas (Historial, Auditoría,
  exportaciones). Todos los comandos son `#[tauri::command(async)]`.
- **Frontend**: `App.tsx` decide entre cargando, primer arranque, login y
  `Shell`. `Shell` tiene menú lateral, área de contenido y barra de estado.
  Cada sección visitada queda montada y se oculta con `display: none`.
- **Diseño**: `design/brisas.json` genera `desktop/src/diseno.css`. Encima
  se aplican `tokyo-day.css` y `tokyo-night.css` después de iniciar sesión.

### Lo que está bien y conviene no romper

- `api/*.ts` es la única capa que llama a `invoke()`; los tipos y errores
  de Rust se mapean en un solo lugar por dominio.
- Lógica pura separada en `*.logica.ts`, con tests al lado (71 archivos
  de test).
- Pantallas y modales con `lazy()`; AG Grid aislado en su propio chunk.
- `useRecargaVisible`: una sección oculta no recarga, queda pendiente y
  recarga una vez al volver.
- `prefers-reduced-motion` respetado en todas las animaciones.
- Perfil release: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`,
  `strip = true`. No cambiar a `panic = "abort"`: `GuiState::core()`
  recupera el mutex envenenado y eso necesita *unwinding*.
- Tokens de diseño con verificación de contraste en CI
  (`design/verificar.mjs`).

## 3. Medidas del bundle

Salida de `npm run build` (sin comprimir / gzip):

| Archivo | Tamaño | Carga |
| --- | --- | --- |
| `ag-grid-vendor` | 943 kB / 264 kB | Al abrir la primera tabla |
| `index` | 490 kB / 141 kB | Siempre, antes del login |
| `react-vendor` | 219 kB / 68 kB | Siempre |
| `Ayuda` | 173 kB / 50 kB | Al abrir Ayuda (incluye el manual) |
| `useRecargaVisible` (chunk compartido) | 46 kB / 15 kB | Con la primera pantalla |
| `index.css` | 39 kB | Siempre |
| `marca.png` | **822 kB** | Siempre (login y pantalla de carga) |
| `lattis-splash.png` | **822 kB** | Siempre (splash) |

`index` incluye `supabase-js` (Realtime), `zod`, `sonner` y
`react-hook-form`. De esos, sólo `zod` y `react-hook-form` hacen falta
antes de iniciar sesión.

## 4. Hallazgos de rendimiento

Ordenados por lo que gana el usuario frente al riesgo del cambio.

### R1. El splash dura 2 segundos aunque la app ya esté lista

- **Dónde**: `desktop/src/App.tsx:100`, `DURACION_MINIMA_SPLASH_MS = 2000`.
- **Qué pasa**: es un piso deliberado (comentario del 2026-09-19: en un
  arranque rápido el splash "no se alcanzaba a leer"). El propio código
  dice que la app "se abre y cierra bastante seguido", así que ese piso se
  paga muchas veces al día.
- **Propuesta**: bajarlo a 0–300 ms. Si el splash pasa demasiado rápido, la
  solución simple es no mostrar splash cuando la app carga en menos de
  ~300 ms, no retenerla.
- **Verificar**: medir en Windows el tiempo entre lanzar el ejecutable y
  ver el login, antes y después.
- **Requiere decisión** (ver 8.2): el piso fue un pedido explícito.

### R2. El logo pesa 822 kB y va dos veces

- **Dónde**: `desktop/src/assets/marca.png` y
  `desktop/public/lattis-splash.png`. Son el mismo PNG de 1254×1254, RGB.
- **Qué pasa**: se muestra a 128 px (login, `.marca-sello`) y a 160 px
  (splash). Son 1,6 MB en el instalador y dos decodificaciones de ~6 MB en
  memoria para una imagen de 160 px, justo en el arranque.
- **Propuesta**: un solo archivo, SVG o PNG de 320×320 (unos 15 kB). Si se
  usa SVG con fondo transparente, también se resuelve el comentario de
  `Sidebar.tsx` sobre el "cuadro blanco" en tema oscuro, y el menú lateral
  podría usar la marca real en vez de las letras "LATTIS" repartidas.
- **Verificar**: `ls -la desktop/dist` después de compilar; login y splash
  se ven igual.

### R3. Cambiar de sección pasa por un fundido de toda el área

- **Dónde**: `desktop/src/App.tsx:841`, `<ViewTransition>` alrededor de las
  secciones, y `cambiarSeccion` con `startTransition` (línea 425).
- **Qué pasa**: el navegador captura el área de contenido completa (una
  grilla AG Grid) y funde la vieja con la nueva. Navegar entre secciones es
  la acción más repetida; el fundido agrega latencia y no informa nada.
- **Propuesta**: quitar `<ViewTransition>`. Las secciones ya están
  montadas: el cambio queda instantáneo.
- **Verificar**: a mano en Windows, sobre todo en equipos modestos.
- **No se midió** el costo real en WebView2; es lectura de código.

### R4. Supabase se carga antes de iniciar sesión

- **Dónde**: `desktop/src/App.tsx:82` importa `./nubeRealtime` de forma
  estática; `nubeRealtime.ts` importa `@supabase/supabase-js`.
- **Qué pasa**: el chunk inicial de 490 kB se analiza y ejecuta antes de
  mostrar el login, aunque Realtime sólo arranca dentro de `Shell`.
- **Propuesta**: `import("./nubeRealtime")` dinámico dentro del efecto de
  `Shell` que hoy llama a `iniciarRealtimeNube`. `emitirActualizacion`
  también sale de ese módulo: hay que moverla o cargarla igual.
- **Verificar**: `npm run build` y comparar el tamaño de `index-*.js`.
  `nubeRealtime.test.ts` debe seguir pasando.

### R5. El buscador de las grillas repite trabajo en cada tecla

- **Dónde**: `desktop/src/componentes/Tabla.tsx:66-87` (`plegar`,
  `quickFilterParser`, `quickFilterMatcher`).
- **Qué pasa**: por cada tecla, `quickFilterMatcher` normaliza (NFD + quitar
  tildes) el texto de **todas** las filas. No hay `cacheQuickFilter` ni
  `useDeferredValue` (se buscó en todo `desktop/src`). En Activos son pocas
  filas; en Historial pueden ser miles.
- **Propuesta**, de menor a mayor cambio:
  1. `useDeferredValue(busqueda)` antes de pasarlo a AG Grid.
  2. `cacheQuickFilter` en la grilla y que cada columna devuelva el texto
     ya plegado desde `getQuickFilterText`, para normalizar una vez por
     fila y no una vez por tecla.
- **Verificar**: `Tabla.test.ts` cubre "Sanches" → "Sánchez"; debe seguir
  pasando. Probar a mano con 6 meses de historial.

### R6. Activos hace 8 llamadas IPC por recarga

- **Dónde**: `desktop/src/api/activosUnificados.ts:151`
  (`listarActivosUnificados`) y `api/activos.ts:82`.
- **Qué pasa**: cuatro clases (contratistas, proveedores, correo, KOF), y
  cada una pide lo local y lo remoto: 8 `invoke`, más `avisos_permanencia`
  aparte. Cada comando toma el `Mutex<AppCore>`. Se repite en cada ingreso,
  salida, aviso en vivo y sincronización.
- **Propuesta**: un comando `listar_activos_unificados` en
  `desktop/src-tauri/src/comandos/` que tome el candado una vez y devuelva
  las cuatro listas (y, si conviene, los avisos de permanencia). El mapeo a
  `ActivoUnificado` puede quedarse en TypeScript.
- **Verificar**: `activosUnificados.test.ts` (520 líneas) define el
  contrato; adaptar los tests al comando nuevo.
- **No se midió**: puede que 8 llamadas locales cuesten pocos milisegundos.
  Medir con el grupo `rendimiento` de la telemetría (`api/invocar.ts`)
  antes de invertir aquí.

### R7. Historial abre con 6 meses

- **Dónde**: `desktop/src/pantallas/HistorialContratistas.tsx` y
  `componentes/SelectorRangoFecha.logica.ts`.
- **Qué pasa**: el rango inicial trae 6 meses por IPC como JSON, acotado
  por `LIMITE_CARGA_COMPLETA_MAXIMO`. La consulta típica de un puesto de
  control es de hoy o de esta semana.
- **Propuesta**: abrir con 7 días o el mes en curso; 6 meses queda como
  acceso rápido.
- **Requiere decisión** (ver 8.3).

### R8. Detalles menores de animación

| Dónde | Qué pasa | Propuesta |
| --- | --- | --- |
| `index.css`, `.login-card` | `backdrop-filter: blur(16px)` sobre una tarjeta de fondo opaco: no se ve y obliga a una capa extra | Quitarlo |
| `index.css`, `.modal-cuerpo` + `Modal.tsx` | Anima `height` con el alto medido por `ResizeObserver`: recalcula el layout en cada cuadro | Animar `grid-template-rows: 0fr → 1fr` en la ficha que se despliega, o aceptar el salto |
| `main.tsx`, `NieveNavidena` | Se importa siempre, aunque sólo dibuja en temporada | Cargarlo con `lazy` sólo si `NAVIDAD_ACTIVA` |
| `index.css` | Duraciones escritas a mano: 0.15 s, 0.22 s, 0.25 s, 0.35 s, 0.4 s, 160 ms, 180 ms, 220 ms | Dos variables: `--duracion` (ya existe, 150 ms) y `--duracion-larga` |

### Lo que se revisó y no hace falta tocar

- **Secciones montadas para siempre**: gasta memoria, pero volver a una
  sección es instantáneo y conserva filtros y scroll. Buen intercambio.
- **Chunk de AG Grid (943 kB)**: ya está aislado, se carga de disco y sólo
  al abrir una tabla. `modulosTabla.ts` registra sólo lo que se usa.
- **`useAvisosPermanencia` cada 60 s**: no redibuja si la respuesta no
  cambió.
- **`parpadeo-conexion` y `girando`**: animan `opacity` y `transform`, que
  no recalculan el layout.

## 5. Hallazgos de UX/UI

Vistos en las capturas a 1440×860. La numeración sigue el orden de impacto.

### U1. El botón "Salida" de cada fila queda fuera de pantalla

- **Dónde**: `desktop/src/pantallas/Activos.tsx:266-359` (columnas).
- **Qué se vio**: a 1440 px de ancho, la columna "Acción" aparece cortada
  en el borde derecho ("AC…" en el encabezado, medio botón en cada fila).
  Los anchos mínimos de las 11 columnas más la casilla suman más que el
  espacio disponible con el menú lateral abierto. En un portátil de
  1366 px será peor.
- **Por qué importa**: es la acción principal por fila de la pantalla
  principal.
- **Propuesta**: aplicar U2 (menos columnas) y fijar "Acción" a la derecha
  (`pinned: "right"`). El comentario de la línea 337 explica por qué no se
  fijó antes (reserva el ancho del scrollbar); con menos columnas ese
  espacio deja de importar.
- **Verificar**: capturas a 1366, 1440 y 1920 px, con el menú abierto y
  colapsado.

### U2. Columnas de Activos: sobran unas y falta la importante

| Columna actual | Problema | Propuesta |
| --- | --- | --- |
| Clase | Repite el filtro de clase; con un filtro puesto, todas las filas dicen lo mismo | Ocultarla por defecto; visible sólo con "Todas" |
| Fecha + Hora | En Activos casi todo es de hoy: la fecha repite el mismo valor en cada fila | Una columna "Ingreso": hora, y fecha sólo si no es hoy |
| *(no existe)* | El dato que se vigila es cuánto lleva adentro; hoy hay que restar a mano | "Tiempo adentro" (`3 h 20 min`), ordenable |
| Dio ingreso | Dato de auditoría, no de operación | Opcional, oculta por defecto |
| Nombre | Se corta: "CARLOS MAURICI…" | Más ancho mínimo; es la columna que identifica a la persona |
| Medio | Se corta: "CAMIN…" | Ícono + placa, o texto corto ("A pie" / placa) |

"Tiempo adentro" debe salir del núcleo o de `fecha_hora_ingreso`; el nivel
de permanencia ya lo calcula `domain::permanencia`. No duplicar esa regla
en TypeScript (ver `docs/auditorias/reglas-duplicadas-escritorio-2026-09-27.md`).

### U3. Filtros con íconos y números, sin texto

- **Dónde**: `Activos.tsx:35-113` (`opcionesFiltroGafete`,
  `opcionesFiltroClase`) y `componentes/SegmentadoOpciones.tsx`.
- **Qué se vio**: el filtro de clase se lee `▤ 12 · ⛑ 12 · ⚙ 0 · ✉ 0 · ✓ 0`.
  El significado sólo aparece al dejar el mouse encima.
- **Propuesta**: pestañas con texto: `Todos 12 · Contratistas 12 ·
  Proveedores 0 · Correo 0 · KOF 0`. Hay espacio una vez que se agrupan los
  botones de tabla (U5).
- **Mismo problema** en: el botón `+` de Contratistas, Empresas y Gafetes
  (debería decir `+ Nuevo contratista`), los dos íconos del modal Salida
  (buscar por persona / por gafete) y el selector de clase de Historial.

### U4. Tres rellenos de acento a la vez

- **Qué se vio**: en Activos, el ítem activo del menú, el filtro "12" y el
  embudo de filtros están los tres rellenos con el color de acento. Además
  el acento pinta la marca "LATTIS" y los títulos de los modales. Ningún
  elemento destaca como "lo siguiente que hay que hacer".
- **Propuesta**: ver la regla de color en 6.2.

### U5. La barra de herramientas cambia de orden entre pantallas

| Pantalla | Izquierda | Centro | Derecha |
| --- | --- | --- | --- |
| Activos | Ingreso, Salida, filtros | — | Buscador, 4 íconos de tabla |
| Historial | Buscador | Selector de clase | Rango, exportar, 4 íconos |
| Contratistas | `+`, buscador | — | 4 íconos |
| Gafetes | `+`, buscador, selector | — | 4 íconos |

- **Propuesta**: un solo orden en todas: acción principal → filtros →
  buscador → menú "Tabla". Los cuatro íconos (filtros por columna, ajustar
  anchos, restablecer anchos, columnas visibles) pasan a un menú único:
  son de configuración ocasional y hoy ocupan el sitio más visible.
- **Dónde**: `componentes/Tabla.tsx:614-753` arma la barra con las props
  `controles`, `accionesCentro` y `accionesDerecha`.

### U6. Dos maneras de filtrar siempre visibles

- **Qué se vio**: buscador global arriba y, además, una fila de filtros por
  columna encendida por defecto. La fila ocupa 38 px y muestra una docena
  de cajas vacías.
- **Dónde**: `Tabla.tsx:329`, `filtrosVisibles ?? true`.
- **Propuesta**: apagada por defecto; el embudo la enciende y la
  preferencia ya se guarda por usuario.

### U7. Grillas con cebra, líneas y resaltado de columna a la vez

- **Dónde**: `Tabla.tsx:122-134` (`temaBrisas`, `oddRowBackgroundColor`) y
  `columnHoverHighlight` en la línea 770.
- **Propuesta**: elegir uno. Con filas de 36 px, sólo líneas horizontales
  finas basta. Quitar `columnHoverHighlight`.

### U8. Estados vacíos sin siguiente paso

- **Dónde**: `Tabla.tsx:180`, `MENSAJE_SIN_FILAS` ("Sin resultados").
- **Propuesta**: que cada pantalla pase su mensaje. Ejemplos: "No hay nadie
  adentro. Ctrl+N registra un ingreso." / "Nada coincide con «X»." con un
  botón "Limpiar búsqueda".

### U9. El estado de conexión no dice qué implica

- **Dónde**: `componentes/BarraNube.tsx`, `BarraNube.logica.ts`.
- **Qué se vio**: "Sin conexión en vivo" en rojo con un punto que late. No
  dice si se puede seguir trabajando.
- **Propuesta**: "Sin conexión: se guarda aquí y se sincroniza al volver",
  en ámbar. El rojo queda para lo que exige una acción (equipo retirado).

### U10. Funciones escondidas tras el clic derecho

- **Dónde**: `componentes/Sidebar.tsx` (arrastrar para reordenar, clic
  derecho para ocultar secciones) y `MenuContextualSecciones.tsx`.
- **Qué pasa**: `main.tsx` desactiva el menú contextual en toda la app, así
  que nadie espera que el clic derecho haga algo en el menú lateral. Con 7
  secciones visibles, reordenar aporta poco.
- **Propuesta** (ver 8.4): o se lleva al menú de usuario como "Personalizar
  menú", o se elimina junto con `@dnd-kit` (tres paquetes) y unas 250
  líneas entre `App.tsx`, `Sidebar.tsx` y `MenuContextualSecciones.tsx`.

### U11. Detalles

| Dónde | Qué pasa | Propuesta |
| --- | --- | --- |
| `Activos.tsx:446-478` | El botón de la salida masiva dice "Confirmar" | "Registrar 3 salidas" |
| `Modal.tsx:139` | El cierre es el carácter `✕` dentro de un `.boton` con borde | Ícono `X` de lucide, botón discreto |
| `index.css:159-193` | Títulos, etiquetas y botones de modal en mayúsculas | Mayúsculas sólo en datos y encabezados de columna (ver 8.5) |
| `VersionFooter.tsx` | "v1.10.9" es también el botón de buscar actualización; nada lo indica | Pasarlo al menú de usuario como "Buscar actualización" |
| `MenuUsuario.tsx` | "Tema: Tokyo Night" usa nombres que el operador no conoce | "Tema: claro / oscuro" |
| `desktop/index.html:2` | `lang="en"` | `lang="es"` |
| Contratistas, columna "Estado" | Vacía cuando todo está bien | Mostrar "Con acceso" en verde: un vacío no es un estado |
| Contratistas, columna "Adentro" | "SÍ" en negrita | El mismo indicador que use Activos (punto de color + texto) |
| Historial, columna "Dispositivo" | Emoji 💻 | Ícono de lucide, como el resto |

## 6. Sistema visual

### 6.1. Hay cuatro paletas

| Dónde | Paleta | Acento |
| --- | --- | --- |
| Splash (`public/splashscreen.html`) | Colores escritos a mano | `#0b7eb0` |
| Login, primer arranque, pantalla de carga | Brisas claro/oscuro (`diseno.css`, según el sistema) | `#203C63` / `#A6BCD9` |
| Shell, tras iniciar sesión | Tokyo Day / Tokyo Night (`useTema`) | `#2959aa` / `#7aa2f7` |
| Móvil | "Kash" (fuera de este alcance) | verde azulado |

**Comprobado en capturas**: el login es blanco con botón azul marino y, al
entrar, la app pasa a gris lavanda con otro azul. El tema Tokyo sólo se
aplica en `Shell` porque depende del usuario (`useTema(sesion.id)`).

**Defecto concreto de Tokyo Day**: los fondos de estado son el color al
10 % sobre el panel. `--advertencia-suave` es `#d7d5d5` sobre un panel
`#e1e2e7`, con filas alternas `#e9e9ed`. En la captura, la fila de "más de
12 h" casi no se distingue de una fila normal; la de "más de 14 h" se ve
apenas. La marca de 3 px a la izquierda es lo único que las delata. En
Tokyo Night sí se distinguen.

`design/verificar.mjs` sólo revisa los temas de `brisas.json`: los Tokyo
viven aparte y no pasan por esa verificación.

### 6.2. Propuesta

**Una sola fuente de color**: `design/brisas.json`, claro y oscuro, desde
el splash hasta el último modal. Valores de partida (pasarlos por
`node design/verificar.mjs` antes de adoptarlos; no se verificaron):

| Rol | Claro | Oscuro |
| --- | --- | --- |
| `fondo` | `#F1F3F6` | `#0F1216` |
| `panel` | `#FFFFFF` | `#181C22` |
| `borde` | `#D5DBE4` | `#2C333D` |
| `texto` | `#1B2430` | `#E8ECF2` |
| `muted` | `#5A6678` | `#A3AEBF` |
| `acento` | `#1D4ED8` | `#6C9BFF` |
| `exito` | `#2F7D4F` | `#7CC79A` |
| `advertencia` | `#B4690E` | `#E0B065` |
| `error` | `#C0392B` | `#F08A80` |

Los fondos `*-suave` deben distinguirse de `panel` **y** de la fila
alterna: comprobarlo en la fila de permanencia, no sólo en un aviso.

**Regla de uso** (para agregar a `design/README.md`):

1. Los neutros llevan la interfaz: fondos, bordes, texto.
2. El acento marca tres cosas y nada más: la acción principal de la
   pantalla, dónde estoy (ítem activo del menú, opción elegida) y el foco
   del teclado. No se usa en títulos, marca ni decoración.
3. Verde, ámbar y rojo significan estado. Siempre con ícono o texto.
4. Como máximo un botón relleno de acento por pantalla o modal.

**Tipografía**: hoy hay 15 tamaños distintos entre `index.css` y los
estilos en línea (0.65, 0.7, 0.72, 0.75, 0.78, 0.8, 0.8125, 0.82, 0.85,
0.88, 0.9, 0.95, 1.05, 1.1 y 1.25 rem). Propuesta: cuatro tokens.

| Token | Tamaño | Uso |
| --- | --- | --- |
| `--texto-xs` | 12 px | Barra de estado, etiquetas de campo, notas |
| `--texto-sm` | 13 px | Celdas de grilla, texto secundario |
| `--texto-base` | 14 px | Controles y texto general (ya es `--fuente-base`) |
| `--texto-titulo` | 18 px | Títulos de modal y de pantalla |

**Espaciado**: ya existe la escala 4/8/12/16/24/32 (`--espacio-*`), pero
`index.css` usa `0.375rem` (6 px) en más de 15 lugares y valores sueltos
como `0.45rem` o `0.55rem`. Llevarlos a la escala.

## 7. Organización interna

| # | Dónde | Qué pasa | Propuesta |
| --- | --- | --- | --- |
| O1 | `App.tsx` (1008 líneas) | `Shell` mezcla preferencias del menú, sincronización, actualizaciones, atajos, modales y layout. Tiene ~15 `useState` | Extraer `useSidebarPreferencias`, `useSincronizacionNube`, `useActualizaciones` y `<ModalesGlobales>`. `Shell` queda con composición y enrutado |
| O2 | 330 `style={{…}}` en `desktop/src` | El estilo vive en tres sitios: `index.css`, Tailwind (sólo en Login y carga) y estilos en línea | Elegir uno. Tailwind ya está instalado y los tokens ya están en `@theme` |
| O3 | `index.css` (1283 líneas) | Todo junto: shell, tabla, modal, chips, interruptores, menús | Un archivo por componente, importados desde `index.css` |
| O4 | `Tabla.tsx` (855 líneas) | Grilla, persistencia de layout, exportación y barra de herramientas | `useLayoutTabla`, `useExportacionTabla` y `<BarraTabla>` |
| O5 | `App.tsx:136-189` | Seis funciones casi iguales de leer/guardar preferencias del menú | Un `usePreferencia(clave, usuarioId, porDefecto)` |
| O6 | `App.tsx:608-662` | Cinco bucles `for` casi iguales para avisar conflictos | Una tabla `[lista, plantilla]` y un solo bucle |

O1 a O4 no cambian nada visible: hacerlos junto con el cambio funcional que
toque cada archivo, no como una reescritura aparte.

## 8. Decisiones pendientes del dueño

Varias propuestas deshacen pedidos explícitos que están documentados en
comentarios del código. No ejecutarlas sin confirmación.

| # | Decisión | Qué deshace | Opciones |
| --- | --- | --- | --- |
| 8.1 | Paleta única | Temas Tokyo (pedido del 2026-09-24) | **A**: sólo Brisas, borrar los Tokyo. **B**: sólo Tokyo, aplicado también a login y splash, con los fondos de estado corregidos. Lo que no conviene es mantener los dos |
| 8.2 | Piso del splash | `DURACION_MINIMA_SPLASH_MS` (2026-09-19) | Quitarlo, bajarlo a ~300 ms o dejarlo |
| 8.3 | Rango inicial de Historial | 6 meses | 7 días, mes en curso o dejarlo |
| 8.4 | Menú lateral reordenable | Función existente | Moverla al menú de usuario o eliminarla |
| 8.5 | Mayúsculas en modales | Pedido del 2026-09-23 | Dejarlas sólo en datos y encabezados, o mantenerlas |
| 8.6 | Fila de filtros por columna | Encendida por defecto | Apagada por defecto o dejarla |
| 8.7 | Columnas de Activos (U2) | Orden y columnas actuales | Aprobar la tabla de U2 o ajustarla |

## 9. Orden de trabajo sugerido

Cada etapa es una rama y un pull request propios.

| Etapa | Contenido | Depende de | Esfuerzo estimado |
| --- | --- | --- | --- |
| 1. Arreglos sin discusión | R2 (logo), R3 (fundido), R8, U1 (fijar "Acción"), `lang="es"`, botón "Registrar N salidas" | Nada | 1 día |
| 2. Rendimiento | R4 (Supabase diferido), R5 (buscador), R1 y R7 si se aprueban | 8.2, 8.3 | 2 días |
| 3. IPC de Activos | R6, **después de medir** con la telemetría de `rendimiento` | Medición | 1–2 días |
| 4. Paleta y tipografía | 6.2 completo, U4, U7 | 8.1, 8.5 | 3–4 días |
| 5. Pantallas | U2, U3, U5, U6, U8, U9, U10, U11 | 8.4, 8.6, 8.7 | 3–4 días |
| 6. Organización | O1–O6, junto a las etapas 4 y 5 | — | Continuo |

### Comprobación mínima de cada etapa

```bash
cd desktop
npm run lint
npm test
npm run build
npm run design:check
```

Y, si se tocó `design/brisas.json`:

```bash
node design/generar.mjs
node design/verificar.mjs
```

Los cambios visuales se comprueban con capturas antes y después (sección 1)
en claro y oscuro, a 1366 y 1920 px. Los de arranque (R1, R2, R4) sólo se
pueden medir de verdad en Windows con el instalador.

## 10. Reglas del proyecto que afectan a quien continúe

Están en `AGENTS.md`; se repiten porque condicionan este trabajo:

- Comunicación, comentarios y documentación en español.
- Para el puesto de control usar "puesto de control", "portería" o "punto
  de acceso"; hay un término prohibido (ver `AGENTS.md`).
- Commits sólo a nombre de `Daniel Quintana`, sin líneas de atribución a
  asistentes.
- Los colores no se escriben a mano en las pantallas: salen de
  `design/brisas.json`. `desktop/src/diseno.css` es un archivo generado.
- Una pantalla no importa a otra pantalla; sólo `api/*.ts` llama a
  `invoke()` (convención al inicio de `App.tsx`).

## 11. Avance

### Fase 1 (rama `perf/escritorio-fase-1`, 2026-11-14)

El dueño aprobó la fase 1 del plan, que incluye R1 y R7 (decisiones 8.2 y
8.3: el piso del splash baja a 300 ms y Historial abre con 7 días).

| Punto | Estado | Qué se hizo |
| --- | --- | --- |
| R1 | Hecho | `DURACION_MINIMA_SPLASH_MS` de 2000 a 300 ms (`App.tsx`) |
| R2 | Hecho | Logo de escritorio a 320 px con paleta de 64 colores: 822 kB → 35 kB cada copia. `scripts/icono_navideno.py` ahora reduce al copiar (`copiar_logo`), así `poner`/`quitar` no devuelven el PNG grande. El de Android queda igual |
| R3 | Hecho | Sin `<ViewTransition>` ni `startTransition` al cambiar de sección. El salto desde una sección oculta pasó de efecto a ajuste durante el render (lo pidió el lint) |
| R4 | Hecho | `nubeRealtime` se importa con `import()` dentro de `Shell`. `emitirActualizacion` y `NubeActualizadaDetalle` se mudaron a `eventosNube.ts` (se reexportan desde `nubeRealtime.ts`). Chunk inicial: 490 kB → 271 kB (gzip 141 → 84 kB) |
| R5 | Hecho | `useDeferredValue` en el buscador, `cacheQuickFilter` en AG Grid y `plegarConCache` (`Tabla.logica.ts`, con tope de 20.000 textos). `plegar` se mudó a `Tabla.logica.ts` |
| R6 | **Replanteado** | La telemetría muestra que el costo está en abrir una conexión SQLite nueva por llamada, no en la cantidad de llamadas (ver sección 12). Prioridad baja |
| R7 | Hecho | `desdeInicialHistorial()` en `SelectorRangoFecha.logica.ts`: Historial y Visitas abren con los últimos 7 días. El atajo "Últimos 7 días" deja `hasta` abierto, igual que "Últimos 6 meses", para no perder lo posterior a medianoche con la app abierta |
| R8 | Parcial | Quitado `backdrop-filter` del login. La nieve ya no cuesta nada con `NAVIDAD_ACTIVA = false` (sale antes de pedir cuadros): no se tocó. La animación de alto del modal se dejó: es un pedido del 2026-09-24 y el costo es chico (sólo el modal) |

Comprobado: `npm run lint`, `tsc --noEmit`, `npm test` (71 archivos, todos
pasan), `npm run build`, `node design/generar.mjs --check` y una prueba de
humo en Chromium con Tauri simulado (Supabase no se pide antes del login;
buscar "karen angeles" encuentra "KAREN DE LOS ÁNGELES"; Historial abre en
"7 DÍAS"; sin errores de página).

**Falta comprobar en Windows** con el instalador: tiempo de arranque (R1,
R2, R4) y que el cambio de sección se sienta inmediato (R3).

## 12. Lo que dice la telemetría (staging, consultada el 2026-11-14)

Tabla `public.telemetria_diagnostico` del proyecto de staging, versiones
`desktop-1.10.x`, últimos 21 días: 3 equipos con el grupo `rendimiento`,
15 días con datos. Tiempos de cada comando medidos en el frontend, IPC
incluido (`api/invocar.ts`), agregados por lote.

### Arranque (`arranque.ms_hasta_ventana`)

| Versión | Arranques | p50 | p90 | Mínimo |
| --- | --- | --- | --- | --- |
| 1.10.7 | 27 | 3,16 s | 4,00 s | 2,81 s |
| 1.10.9 | 2 | 3,30 s | 3,32 s | 3,27 s |

Ningún arranque baja de 2,8 s: el piso de 2 s del splash (R1) es la mayor
parte. Con el piso en 300 ms, lo esperable es ~1,2–1,8 s; **medirlo** con
los primeros arranques de la versión que lleve la fase 1.

Anomalía: en 1.10.8 el p50 da 5.830.833 ms (~97 min, 9 arranques). Parece
que el reloj sigue corriendo mientras la ventana no se muestra (equipo
suspendido, o la app abierta oculta). Revisar cómo se mide antes de usar
esa métrica para comparar.

### Recarga de Activos (R6)

5.058 recargas en 21 días (~110 por día y equipo). Las 8 llamadas van en
paralelo, así que la recarga dura lo que la más lenta:

| Comando | Conexión | p50 | p90 |
| --- | --- | --- | --- |
| `listar_ingresos_activos` y otras 3 locales | `state.core()` | 17–19 ms | 58–71 ms |
| `listar_ingresos_remotos` y otras 3 remotas | `conexion_secundaria()` **nueva en cada llamada** | 78–79 ms | 146–149 ms |

- Una recarga tarda **~80 ms (p50) y ~150 ms (p90)**. Por debajo de 100 ms
  no se nota; en el p90 apenas.
- Picos raros: 21 de 4.348 lotes (0,5 %) pasan de 500 ms y 8 de 2 s; 5 de
  esos 8 coinciden con una sincronización a menos de 2 minutos.
- Lo caro no es la cantidad de llamadas sino **abrir una conexión SQLite
  nueva en cada una** (`GuiState::conexion_secundaria`): se lee el esquema
  y la caché de páginas arranca vacía, con descifrado ChaCha20. La clave va
  en crudo (`x'…'`), así que no hay derivación de clave. Prueba indirecta:
  `aviso_bandeja_salida`, que sólo cuenta pendientes, tarda 25 ms (p50).

**Conclusión: R6 tal como estaba planteado (un solo comando) no vale la
pena.** La alternativa de menos piezas es una conexión de lectura
persistente en `GuiState` (un `Mutex<Connection>` abierto una vez) para
los listados remotos, `aviso_bandeja_salida` y `listar_historial_sitio`.
Ganancia estimada: ~50 ms por recarga de Activos. Prioridad baja.

### Otros comandos

| Comando | Llamadas | p50 | p90 | Nota |
| --- | --- | --- | --- | --- |
| `aviso_bandeja_salida` | 63.937 | 25 ms | 46 ms | Cada 15 s en `BarraNube.tsx`; con la conexión persistente bajaría a casi nada |
| `listar_historial` + `listar_historial_sitio` | 2.674 | 72 + 183 ms | 134 + 346 ms | Con 6 meses. El arranque en 7 días (R7) debería bajarlo mucho; medir |
| `sincronizar_con_nube` | 499 | 1,2 s | 2,0 s | Red; máximo 28 s. 8 errores |
| `sesion_realtime_nube` | 1.033 | 3 ms | 8 ms | 338 errores (33 %): todos de un solo equipo sin red, ver abajo |
| `login` | 79 | 406 ms | 1,0 s | 33 errores (probablemente contraseñas mal escritas) |

### Interfaz

- Cambio de pantalla hasta el primer cuadro: 23–31 ms (p50) y 27–74 ms
  (p90) en todas las secciones. La navegación ya era rápida; R3 quita el
  fundido, que esta métrica no capturaba.
- Tareas largas del hilo principal (`ui_bloqueos`): p50 ~55 ms y p90
  ~90 ms en Activos; sólo 2 pasaron de 200 ms en todo el período. La
  interfaz no se traba.

### Qué cambia en las prioridades

1. El arranque es lo único que el operador siente todos los días: la
   fase 1 ya lo ataca (R1, R2, R4). Falta medirlo en la versión nueva.
2. R6 baja a prioridad baja y cambia de forma (conexión persistente, no
   comando único).
3. El 33 % de errores de `sesion_realtime_nube` no es un fallo de la app: ver abajo.
4. El resto del rendimiento está bien: lo que queda por ganar está en la
   UX (fase 3), no en la velocidad.

### Errores de la conexión en vivo (`sesion_realtime_nube`), escritorio y móvil

Últimos 30 días, todas las versiones:

| Plataforma | Equipos con error | Llamadas | Errores | Tipo |
| --- | --- | --- | --- | --- |
| Escritorio | 1 de 3 (`a016c1b9…`) | 1.715 | 872 (51 %) | `sin_red` (528 de versiones viejas sin clasificar) |
| Escritorio | los otros 2 | — | 0 | — |
| Móvil | 4 de 9 | 2.872 | 92 (3 %) | Sobre todo `no_autenticado` (72); `interno` 7, `sin_red` 4 |

- **No es un fallo de la app de escritorio.** Los errores salen de un solo
  equipo y llegan a razón de ~60 por hora: es el reintento con espera
  máxima de 60 s de `nubeRealtime.ts` (`REINTENTO_TOPE_MS`) mientras la PC
  no tiene red, a veces durante horas (por ejemplo, de 9 h a 17 h). En sólo
  16 de 863 minutos con error había otra llamada a la nube funcionando en
  el minuto de al lado: son los momentos en que la red se cae o vuelve.
- **Hecho (2026-11-14, aprobado por el dueño):** con `navigator.onLine`
  en `false`, `nubeRealtime.ts` ya no reintenta: espera el evento `online`
  y entonces conecta enseguida, también si estaba en medio de una espera
  entre reintentos. Contexto del dueño: esa PC se conecta por el internet
  compartido de un teléfono, y hay días en que nadie se lo comparte. Si el
  teléfono comparte sin datos (`onLine` en `true` pero sin internet), siguen
  los reintentos normales.
- **Lo que sí conviene revisar es la red de ese puesto**: identificar el
  equipo `a016c1b9…` (ver `sesion_inicio`/`identidad_equipo`). Mientras no
  hay red, la PC no recibe avisos en vivo; la sincronización periódica
  recupera al volver.
- **Móvil:** tasa baja (3 %), casi toda en un teléfono (`b5e22be5…`, 66 de
  `no_autenticado`): la sesión no estaba lista al conectar y el reintento
  la resuelve. No requiere acción por ahora.

### Fase 3 (rama `ux/escritorio-fase-3`, sobre `perf/escritorio-fase-1`, 2026-11-14)

Decisiones del dueño: 8.7 propuesta completa de columnas de Activos, 8.6
filtros por columna apagados por defecto, 8.4 llevar la personalización del
menú al menú de la cuenta. Y una regla nueva, que manda sobre U3:

> **Toda la barra de herramientas en una sola línea.** Por eso los filtros
> de clase y de gafete y la exportación son íconos con número; su nombre
> está en el título (al pasar el mouse). Una segunda línea sólo con una
> justificación muy fuerte.

Se probó poner las clases con nombre en una fila de pestañas aparte y se
descartó por esa regla. Todas las pantallas se verificaron a 1366 px: una
sola línea.

| Punto | Estado | Qué se hizo |
| --- | --- | --- |
| U1 | Hecho | «Acción» fijada a la derecha en Activos: el botón «Salida» siempre se ve |
| U2 | Hecho | Activos: «Clase» sólo con «Todas» (`columnasOcultasPorPantalla`, no se mueve de lugar), «Ingreso» (hora, y fecha si no es hoy), «Tiempo» (tiempo adentro, se recalcula cada minuto con `refrescarColumnas`), «Dio ingreso» oculta por defecto. Entra a 1366 px sin desplazamiento horizontal |
| U3 | **Revertido en parte** | Por la regla de una línea, las clases, el gafete y la exportación siguen con íconos. Sí llevan texto los botones de crear («Nuevo contratista», «Nueva empresa», «Nuevo gafete»), donde sobra lugar |
| U5 | Hecho | Misma barra en todas las pantallas: acción y filtros a la izquierda, buscador y menú «Tabla» a la derecha. Los cuatro íconos de configuración de la grilla pasaron a un solo botón «Tabla» |
| U6 | Hecho | Fila de filtros por columna apagada por defecto. Los layouts viejos la tenían guardada en `true` sin que nadie la eligiera: sólo cuenta si la persona la tocó (`filtrosElegidos`) |
| U7 | Hecho | Sin cebra ni resaltado de columna |
| U8 | Hecho | Mensaje propio con la tabla vacía: texto buscado, filtros, o el que pasa cada pantalla (`mensajeVacio`). Los avisos de AG Grid, suprimidos (`suppressOverlays`) |
| U9 | Hecho | «Sin internet — se guarda en esta PC» y «Sin conexión en vivo» en ámbar, no en rojo |
| U10 | Hecho | «Personalizar menú» en el menú de la cuenta; el clic derecho sigue; pista de arrastrar |
| U11 | Hecho en parte | «Registrar N salidas», cierre de modal con ícono X, «Buscar actualización» en el menú de la cuenta (la versión del pie es sólo texto), tema «Claro»/«Oscuro», `lang="es"`. **Corrección:** la columna «Estado» de Contratistas no sale vacía con datos reales (dice «Puede entrar»): fue un efecto de los datos simulados. «SÍ» en negrita de «Adentro» y emoji de «Dispositivo» en Historial: resueltos después (ver «Conexión persistente y detalles») |

Comprobado: lint, `tsc --noEmit`, 71 archivos de test (537 tests),
`npm run build`, y capturas con Tauri simulado a 1366, 1440 y 1920 px en
claro y oscuro.

**Falta comprobar en Windows** con datos reales: anchos de columnas de
Activos con nombres largos y que «Tiempo» se actualice cada minuto.

### Interfaz en inglés y español: fuera de prioridades

**Decisión del dueño (2026-11-14): no es prioridad.** No retomarlo sin que lo pida. Se deja la medición por si algún día se retoma. Medición rápida (2026-11-14): ~750
textos en español en 73 archivos del frontend; ~800 mensajes en el núcleo
Rust (errores, avisos, textos de Excel/PDF), que hoy llegan ya redactados
en español; y el manual de Ayuda (`docs/manual`). Complejidad estimada 7/10
(ver la conversación del 2026-11-14 para el desglose).

### Fase 2 (rama `ux/escritorio-fase-2`, sobre `ux/escritorio-fase-3`, 2026-11-14)

Decisiones del dueño: 8.1 la paleta de toda la app es **Tokyo Day / Night**
(no Brisas); 8.5 mayúsculas sólo en datos y encabezados. El pull request
de las fases 1, 3 y 2 se abre al final, no todavía.

| Punto | Estado | Qué se hizo |
| --- | --- | --- |
| Paleta única | Hecho | Tokyo pasa a `design/brisas.json` (temas `tokyo-day` y `tokyo-night`) y se genera en `desktop/src/tokyo.css` y `web/src/tokyo.css`; se borraron los `tokyo-*.css` escritos a mano. `web-visitas` sigue con `light`/`dark` |
| Contraste | Hecho | Antes Tokyo no se verificaba y fallaba 8 combinaciones en el claro y 2 en el oscuro. Ajuste mínimo de `muted`, `borde-fuerte` y colores de estado. `verificar.mjs` revisa los 4 temas (128 combinaciones) |
| Filas de permanencia | Hecho | Fondos de estado teñidos con el amarillo y el rosa de Tokyo: en el claro, la fila de «más de 12 h» tenía contraste 1,13 con una fila normal. El verificador ahora exige que esos fondos se distingan del panel |
| Login y splash | Hecho | `temaInicial()`: el último tema usado en esta PC o el del sistema, aplicado antes del primer render. El splash usa el acento Tokyo (sigue al sistema: la CSP no permite scripts en línea) |
| Regla de acento | Hecho | Escrita en `index.css`. Títulos de modal y marca a color de texto. Los demás usos del acento como texto eran estados elegidos, enlaces o acciones: se quedan |
| Mayúsculas | Hecho | Fuera de títulos, etiquetas y botones de modal, del título del login y de los botones del período. Se quedan en los datos, los encabezados de grilla y los rótulos pequeños de sección |
| Tipografía | Hecho | 4 tokens (`--tam-xs`, `--tam-sm`, `--tam-base`, `--tam-titulo`) en lugar de 15 tamaños: 70 reemplazos. Excepciones documentadas: letra del engranaje y unidad bajo la marca |
| Regresión de la fase 3 | Corregida | El selector de período se abría hacia la izquierda y quedaba cortado al estar en el extremo izquierdo; ahora se abre hacia abajo |

Comprobado: lint, `tsc --noEmit`, 71 archivos de test (540 tests),
`npm run build` del escritorio, `generar.mjs --check`, `verificar.mjs` y
capturas en claro y oscuro (login, Activos, modal, período).

**No comprobado aquí:** el panel web no se pudo instalar en la sesión (la
política de red bloquea `cdn.sheetjs.com`, de donde baja `xlsx`). Su único
cambio es importar el `tokyo.css` generado, idéntico al del escritorio; lo
valida el CI (`web.yml`). Los colores de estado del panel cambian igual que
los del escritorio.

**Pendiente:** el nombre del sistema de diseño sigue siendo «Brisas»
aunque la paleta de las apps sea Tokyo; renombrarlo toca el panel web y la
página de visitas y no aporta al usuario.

### Conexión persistente y detalles (rama `mejoras/conexion-y-detalles`, sobre `ux/escritorio-fase-2`, 2026-11-14)

Pedido del dueño: «la conexión y lo visual»; el inglés queda fuera de
prioridades.

**Conexión de lectura persistente (R6 replanteado).**

- `GuiState::con_lectura` (`desktop/src-tauri/src/estado.rs`): una conexión
  de sólo lectura (`abrir_conexion_secundaria`, `PRAGMA query_only`) que se
  abre la primera vez y se reutiliza. Si una consulta falla, la conexión se
  descarta y la próxima llamada abre otra. Orden de candados: `con_lectura`
  primero y después `core()`, nunca al revés.
- La usan sólo las cinco lecturas chicas y frecuentes:
  `aviso_bandeja_salida` (cada 15 s) y los cuatro listados remotos de
  Activos (`listar_ingresos_remotos`, `..._proveedor_remotos`,
  `..._correo_remotos`, `listar_prestamos_gafete_provisional_remotos`), que
  además pasan a `prepare_cached`. Los historiales, la auditoría y las
  escrituras siguen con `conexion_secundaria()`: una consulta larga sobre la
  conexión compartida haría esperar a Activos.
- La base sólo se borra al arrancar, antes de que exista `GuiState`
  (`abrir_sin_base_descartada` en `lib.rs`), así que una conexión abierta
  no impide borrarla.
- Medición con el núcleo real y una base SQLite3MC (en este servidor):
  abrir una conexión por llamada 1,6 ms contra 0,007 ms reutilizándola. En
  las PCs del puesto pesa más (Windows y antivirus al abrir archivos; la
  telemetría mostraba 25 ms por llamada). **Confirmar con la telemetría de
  la versión instalada**: `aviso_bandeja_salida` y `listar_ingresos_remotos`
  deberían bajar mucho.

Comprobado: `cargo check` y `cargo clippy --all-targets` contra
`x86_64-pc-windows-gnu` sin errores ni advertencias nuevas (el crate sólo
compila para Windows); la lógica de reutilizar y descartar
(`con_conexion_reutilizada`) tiene 3 tests, que se corrieron con el mismo
código en un proyecto aparte porque el crate no compila en Linux.

**Detalles visuales.** «Adentro» de Contratistas con el indicador de punto
de color (`CeldaAdentro`); «Dispositivo» de Historial con ícono de lucide y
el nombre («PC» / «Celular») para el filtro y el Excel/PDF, en vez de un
emoji.

### Panel web con el mismo diseño (rama `ux/panel-web-mismo-diseno`, sobre `mejoras/conexion-y-detalles`, 2026-11-14)

Pedido del dueño: llevar al panel web los mismos cambios visuales, «que
prácticamente son una calca». No es copia literal (la grilla del panel
tiene paginación en el servidor y pantallas propias), así que se adaptó
pieza por pieza:

| Escritorio | Panel web |
| --- | --- |
| Menú «Tabla», filtros apagados, sin cebra ni resaltado de columna | Igual (`Tabla.tsx`, `temaTabla.ts`; el tema también lo usa el lienzo) |
| Mensaje propio con la tabla vacía, buscador con caché | Igual en las tablas que filtran en el navegador; las paginadas en el servidor no cambian |
| Misma barra en una línea: acción y filtros a la izquierda, buscador y «Tabla» a la derecha | Igual en Activos, Contratistas, Usuarios, Dispositivos, Empresas proveedoras, Sesiones, Telemetría e Historial. En Activos el buscador iba primero (pedido del 2026-10-26); el dueño pidió igualar al escritorio |
| «Nuevo contratista» con ícono | «Nuevo contratista», «Nuevo usuario», «Nuevo dispositivo» |
| Activos: «Ingreso» corto, «Tiempo», «Clase» sólo con «Todas», «Dio ingreso» opcional | Igual con «Entrada» y «Tipo» (el panel ya tenía «Tiempo adentro»; es de sólo lectura, sin «Salida») |
| Período que se abre hacia abajo | Igual |
| Tokyo en el login, «Claro»/«Oscuro», acento, mayúsculas, 4 tamaños, X del modal | Igual. La pantalla del Asistente conserva su tipografía propia |
| «Personalizar menú» en el menú de la cuenta | Igual (el panel no tiene «Buscar actualización») |
| «Dispositivo» con ícono | Igual (`IconoDispositivo`) |

Comprobado: `tsc --noEmit`, lint, 67 archivos de test (508 tests), `npm run
build` y capturas con una sesión simulada (todas las peticiones a Supabase
interceptadas, ninguna salió de la sesión) en claro y oscuro. Para
instalar las dependencias se usó un sustituto local de `xlsx`, cuyo
servidor está bloqueado en esta sesión (no se subió nada de eso); el
botón Excel no se pudo probar aquí y lo valida el CI.
