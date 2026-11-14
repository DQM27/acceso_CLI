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
