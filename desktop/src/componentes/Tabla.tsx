import "./modulosTabla";
import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AgGridReact } from "ag-grid-react";
import { themeQuartz } from "ag-grid-community";
import { AG_GRID_LOCALE_ES } from "@ag-grid-community/locale";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Sheet } from "lucide-react";
import type {
  ColDef,
  GetRowIdParams,
  ITooltipParams,
  RowClassParams,
  RowClickedEvent,
  ColumnMovedEvent,
  ColumnPinnedEvent,
  ColumnResizedEvent,
  ColumnState,
  GridReadyEvent,
  SortChangedEvent,
  TextMatcherParams,
} from "ag-grid-community";
import { useSeccionActiva } from "../contexto/BarraEstadoContexto";
import { useUsuarioId } from "../contexto/SesionContexto";
import { ListaFlotante, useListaFlotante } from "./ListaFlotante";
import FiltroFechaTabla from "./FiltroFechaTabla";
import { guardarCsv } from "../api/exportacion";

/**
 * Tema y comportamiento compartido de TODAS las tablas de la app — un solo
 * punto para cambiar cómo se ven/comportan las grillas. Usa las mismas
 * custom properties que el resto de la app (index.css) en vez de colores
 * fijos, para que la grilla siga el tema claro/oscuro del sistema en lugar
 * de quedar siempre oscura sin importar el resto de la interfaz.
 *
 * Deliberadamente NO expone cualquier prop de AgGridReact. Mostrar/ocultar
 * columnas es capacidad permanente (no un flag opcional) porque ya es un
 * patrón establecido en varias pantallas de comandos/TUI (Contratistas,
 * Historial). Selección múltiple sí es opt-in (`seleccionMultiple`), porque
 * no toda pantalla la necesita. Otras funciones de grilla (paginación
 * server-side, exportar) se agregan recién cuando una pantalla real las
 * necesite — no antes.
 */
/** Espejo en JS de `PLEGAR` (SQLite, `database/schema.rs`) -- mismo criterio
 * de "sin tildes/mayúsculas no importa" que ya tiene el buscador de Rust
 * (`database/search.rs`). El `quickFilterText` propio de AG Grid sólo hace
 * `.toUpperCase()`, sin tocar diacríticos -- sin esto, buscar "Sanches" en
 * la grilla no encontraba a "Sánchez" aunque el mismo texto sí funcionara en
 * el buscador del modal "Nuevo ingreso" (hallazgo real del usuario,
 * 2026-09-21: dos buscadores con reglas distintas para el mismo dato). */
function plegar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase();
}

/** `quickFilterParser`/`quickFilterMatcher` de AG Grid -- el parser pliega
 * cada palabra tecleada, el matcher pliega el texto agregado de la fila
 * antes de comparar (AG Grid ya lo entrega en mayúsculas, `plegar` sólo le
 * saca los acentos). Mismo AND implícito que el default de AG Grid: todas
 * las palabras tienen que aparecer en algún lado de la fila. */
function quickFilterParser(texto: string): string[] {
  return plegar(texto)
    .split(" ")
    .filter((parte) => parte.length > 0);
}

function quickFilterMatcher(partes: string[], textoFila: string): boolean {
  const plegado = plegar(textoFila);
  return partes.every((parte) => plegado.includes(parte));
}

/** Mismo problema que el quick filter de arriba, pero en el filtro POR
 * COLUMNA (`floatingFilter`, `agTextColumnFilter`) -- es código
 * completamente distinto dentro de AG Grid, con su propio "contains" que
 * tampoco separa por palabras ni ignora tildes (hallazgo real del usuario,
 * 2026-09-21: probó "Carlos Sa" en el filtro de la columna Nombre, no en el
 * buscador de arriba, y tampoco encontraba "Carlos Mauricio Sánchez"). Sólo
 * `contains`/`notContains` parten por palabras -- el resto de las opciones
 * (equals, startsWith, etc.) son comparaciones de una sola frase, partirlas
 * no tendría sentido. */
function textMatcher({ filterOption, value, filterText }: TextMatcherParams): boolean {
  if (filterText == null) return true;
  const valorPlegado = plegar(String(value ?? ""));
  const textoPlegado = plegar(filterText);
  switch (filterOption) {
    case "notContains":
      return !valorPlegado.includes(textoPlegado);
    case "equals":
      return valorPlegado === textoPlegado;
    case "notEqual":
      return valorPlegado !== textoPlegado;
    case "startsWith":
      return valorPlegado.startsWith(textoPlegado);
    case "endsWith":
      return valorPlegado.endsWith(textoPlegado);
    case "contains":
    default:
      return textoPlegado
        .split(" ")
        .filter((parte) => parte.length > 0)
        .every((parte) => valorPlegado.includes(parte));
  }
}

const temaBrisas = themeQuartz.withParams({
  backgroundColor: "var(--panel)",
  foregroundColor: "var(--texto)",
  headerBackgroundColor: "var(--panel-suave)",
  headerTextColor: "var(--muted)",
  borderColor: "var(--borde)",
  fontFamily: "var(--fuente)",
  accentColor: "var(--acento)",
  selectedRowBackgroundColor: "var(--acento-suave)",
  oddRowBackgroundColor: "var(--campo-fondo)",
  borderRadius: "var(--radio-chico)",
  wrapperBorderRadius: "var(--radio)",
});

const columnaPorDefecto: ColDef = {
  sortable: true,
  resizable: true,
  minWidth: 90,
  // Centrado por defecto (encabezado y dato) en las 4 grillas — la
  // columna de nombre es la excepción explícita, cada pantalla la anula
  // con `cellStyle: { textAlign: "left" }` (el encabezado se queda
  // centrado igual, sólo el dato cambia).
  headerClass: "columna-centrada",
  cellStyle: { textAlign: "center" },
  // Texto completo al pasar el mouse, pero sólo en celdas cortadas
  // ("KAREN DE LOS ANGELE…") -- ver `tooltipShowMode="whenTruncated"` en
  // la grilla. Sólo texto/números: una celda con componente propio (botón
  // "Salida", interruptor) no tiene nada que mostrar.
  tooltipValueGetter: textoTooltip,
};

/** Lo que muestra el tooltip de una celda -- el valor ya formateado (ej.
 * "S/G", "23/09/2026") si la columna tiene `valueFormatter`, si no el valor
 * crudo; `undefined` (sin tooltip) para cualquier cosa que no sea texto o
 * número. */
export function textoTooltip({ valueFormatted, value }: ITooltipParams): string | undefined {
  if (typeof valueFormatted === "string" && valueFormatted !== "") return valueFormatted;
  if (typeof value === "string" && value !== "") return value;
  if (typeof value === "number") return String(value);
  return undefined;
}

/** Un clic dentro de un botón, interruptor o campo de la celda no debe
 * marcar/desmarcar la fila -- ej. el botón "Salida" de Activos ya hace su
 * propia acción. */
export function clicEnControlInteractivo(objetivo: EventTarget | null | undefined): boolean {
  return objetivo instanceof Element && objetivo.closest("button, input, select, a, label") !== null;
}

/** Comparador del filtro de fecha para columnas cuyo valor es "AAAA-MM-DD"
 * (`fechaLocalYMD`, todas las columnas de fecha de la app) -- el valor se
 * guarda así para que ordene bien como texto; lo que se ve en pantalla es
 * DD/MM/AAAA (`valueFormatter`). Un valor que no es fecha (ej. "Activo" en
 * "Fecha salida") cuenta como anterior a cualquier fecha. */
export function compararFechaYMD(filtroMedianoche: Date, valorCelda: unknown): number {
  if (typeof valorCelda !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(valorCelda)) return -1;
  const [anio, mes, dia] = valorCelda.split("-").map(Number);
  const celda = new Date(anio, mes - 1, dia).getTime();
  const filtro = filtroMedianoche.getTime();
  return celda < filtro ? -1 : celda > filtro ? 1 : 0;
}

/** `type: "fecha"` / `type: "numero"` en la definición de una columna
 * (ver `columnasConVisibilidad`): filtro de fecha (antes, después, entre...)
 * o de número (mayor que, menor que...) en vez del de texto. Sólo aplica con
 * los filtros por columna visibles. */
const FILTRO_FECHA: ColDef = {
  filter: "agDateColumnFilter",
  dateComponent: FiltroFechaTabla,
  filterParams: {
    comparator: compararFechaYMD,
    inRangeFloatingFilterDateFormat: "DD/MM/YYYY",
  },
};

const FILTRO_NUMERO: ColDef = {
  filter: "agNumberColumnFilter",
  filterParams: {},
};

const MENSAJE_SIN_FILAS = `<span style="color: var(--muted); font-size: 0.9rem;">Sin resultados</span>`;

const columnaPorDefectoConFiltro: ColDef = {
  ...columnaPorDefecto,
  filter: true,
  floatingFilter: true,
  filterParams: { textMatcher },
};

export interface EstadoGuardado {
  ocultas: string[];
  columnas: ColumnState[];
  /** `undefined` en layouts guardados antes de que existiera esta opción —
   * se toma como visible (comportamiento de siempre) para no ocultarle a
   * nadie los filtros sin que lo haya pedido. */
  filtrosVisibles?: boolean;
}

// v2: el layout guardado incluye `pinned` por columna — al sacar el pin
// fijo de Acción (Activos) del código, un layout viejo lo seguía trayendo
// de vuelta desde acá. Subir la versión descarta ese estado guardado
// obsoleto en vez de tener que migrarlo a mano.
export function claveAlmacenamiento(id: string): string {
  return `tabla:${id}:v2`;
}

export function leerEstadoGuardado(id: string | undefined): EstadoGuardado | null {
  if (!id) return null;
  try {
    const crudo = localStorage.getItem(claveAlmacenamiento(id));
    return crudo ? (JSON.parse(crudo) as EstadoGuardado) : null;
  } catch {
    return null;
  }
}

/** Namespacea el id de grilla por usuario logueado — cada quien guarda su
 * propio layout (orden, ancho, ocultas) bajo la misma pantalla sin pisar el
 * de otro usuario en la misma máquina. `usuarioId` ausente (fuera de una
 * sesión) deja el id tal cual — mismo comportamiento de siempre. */
function idPorUsuario(id: string | undefined, usuarioId: number | null): string | undefined {
  if (!id || usuarioId == null) return id;
  return `u${usuarioId}:${id}`;
}

/** Identidad de una columna para visibilidad/orden — `colId` si está
 * explícito (ej. dos columnas que leen el mismo `field`, como Fecha/Hora),
 * si no el `field`. Mismo criterio que usa AG Grid internamente para su
 * propio `getColumnState`. */
export function identidad(columna: ColDef<unknown>): string | undefined {
  if (typeof columna.colId === "string") return columna.colId;
  if (typeof columna.field === "string") return columna.field;
  return undefined;
}

export interface TablaProps<T> {
  columnas: ColDef<T>[];
  filas: T[];
  /** Controles propios de la pantalla (ej. buscador, "+ Nuevo…") — se
   * muestran en la misma línea que "Columnas ▾", a la izquierda. */
  controles?: ReactNode;
  /** Igual que `controles`, pero a la derecha, junto a "Columnas ▾" (ej. un
   * botón de acción que tiene más sentido cerca del selector que mezclado
   * con el buscador de la izquierda). */
  accionesDerecha?: ReactNode;
  /** Texto de búsqueda global (una sola caja, busca en todas las columnas)
   * — alternativa a `filtrosPorColumna` para listas donde un filtro por
   * columna es más de lo que hace falta. La pantalla es dueña del estado
   * del input; esto sólo se lo pasa a AG Grid (`quickFilterText`). */
  busqueda?: string;
  /** Checkbox por fila + checkbox de encabezado para seleccionar varias a la
   * vez. Opcional (no toda pantalla necesita selección múltiple) — cuando se
   * activa, `onSeleccionCambia` avisa a la pantalla qué filas quedaron
   * marcadas para que ella decida qué hacer con eso. */
  seleccionMultiple?: boolean;
  onSeleccionCambia?: (filas: T[]) => void;
  /** Se dispara cuando el usuario edita una celda editable (ej. un checkbox
   * de columna booleana) — entrega la fila completa ya actualizada para que
   * la pantalla decida cómo persistirla. */
  onCeldaEditada?: (fila: T) => void;
  /** Doble click en una fila — pensado para abrir edición. */
  onFilaDobleClic?: (fila: T) => void;
  /** Filtro por columna (fila de filtros bajo el encabezado) en vez del
   * `controles` propio de la pantalla — para listas que se cargan enteras
   * una vez y se filtran del lado del cliente (ej. Activos), a diferencia
   * de pantallas como Contratistas que filtran contra el servidor. */
  filtrosPorColumna?: boolean;
  /** Identificador estable de esta grilla (ej. "activos", "contratistas").
   * Habilita persistir en localStorage el orden, ancho, orden de columnas
   * (sort) y cuáles están ocultas — sin esto la grilla siempre arranca con
   * el layout por defecto. Cada pantalla usa su propio id, así que el
   * layout de una no pisa el de otra. */
  id?: string;
  /** La pantalla todavía está trayendo datos. Muestra "Cargando…" dentro de
   * la grilla sólo mientras no haya ninguna fila: los refrescos posteriores
   * (Realtime, pulso de sincronización) mantienen las filas viejas a la
   * vista en vez de taparlas con un aviso cada vez. */
  cargando?: boolean;
  /** Muestra un botón "CSV" junto a "Columnas ▾" que exporta lo que la
   * grilla tiene visible ahora (filas filtradas y ordenadas, columnas
   * visibles). Es el nombre sugerido del archivo, sin extensión. */
  nombreExportacion?: string;
  /** Identidad estable de cada fila. Con esto, al refrescar los datos AG
   * Grid reconoce la misma fila en vez de redibujar todo, y hace destellar
   * las celdas que cambiaron (ej. una salida que llega por Realtime). */
  idFila?: (fila: T) => string;
  /** Clase CSS extra para una fila según sus datos (ej. resaltar a quien
   * lleva más de 12 horas adentro). Como puede depender de la hora actual,
   * la grilla se redibuja sola cada minuto mientras esto esté puesto. Debe
   * ser una función estable (definida fuera del componente). */
  claseFila?: (fila: T) => string | undefined;
}

/** Mango imperativo opcional (`ref`) para que la pantalla pida datos que
 * viven adentro de la grilla sin tener que duplicar su estado — hoy "las
 * filas que quedaron visibles tras el filtro por columna" y "qué columnas
 * están visibles ahora (selector Columnas ▾)", que usa Historial para
 * exportar exactamente lo que se ve en pantalla (AG Grid filtra filas y
 * oculta columnas del lado del cliente; `AppCore` no tiene forma de saber
 * ninguna de las dos cosas por su cuenta). */
export interface TablaHandle<T> {
  filasFiltradas: () => T[];
  /** Identidades (`colId`/`field`) de las columnas visibles ahora mismo, en
   * el orden real de la grilla — el que queda después de que el usuario
   * arrastra columnas para reordenarlas, no el orden fijo en el código. */
  columnasVisibles: () => string[];
  /** Exporta a CSV lo visible, igual que el botón propio de la grilla --
   * para pantallas que ubican su botón en otro lugar (ej. Historial, junto
   * a Excel/PDF) en vez de usar `nombreExportacion`. */
  exportarCsv: (nombre: string) => Promise<void>;
}

function TablaBase<T>(
  {
    columnas,
    filas,
    controles,
    accionesDerecha,
    busqueda,
    seleccionMultiple,
    onSeleccionCambia,
    onCeldaEditada,
    onFilaDobleClic,
    filtrosPorColumna,
    id,
    cargando,
    nombreExportacion,
    idFila,
    claseFila,
  }: TablaProps<T>,
  ref: React.ForwardedRef<TablaHandle<T>>,
) {
  const usuarioId = useUsuarioId();
  const idGrilla = idPorUsuario(id, usuarioId);
  const [ocultas, setOcultas] = useState<Set<string>>(
    () => new Set(leerEstadoGuardado(idGrilla)?.ocultas ?? []),
  );
  const [filtrosVisibles, setFiltrosVisibles] = useState(
    () => leerEstadoGuardado(idGrilla)?.filtrosVisibles ?? true,
  );
  const [selectorAbierto, setSelectorAbierto] = useState(false);
  const apiRef = useRef<GridReadyEvent<T>["api"] | null>(null);
  const { campoRef: selectorRef, posicion: posicionSelector } = useListaFlotante(selectorAbierto);
  const popoverRef = useRef<HTMLDivElement>(null);

  // Mismo mecanismo que `SelectorRangoFecha`: cierra al clickear afuera del
  // botón y del popover (el popover vive en un portal a `document.body`, así
  // que un click "afuera" del árbol de este componente no lo cierra solo).
  useEffect(() => {
    if (!selectorAbierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (selectorRef.current?.contains(objetivo) || popoverRef.current?.contains(objetivo)) return;
      setSelectorAbierto(false);
    }
    document.addEventListener("mousedown", alHacerClicAfuera);
    return () => document.removeEventListener("mousedown", alHacerClicAfuera);
  }, [selectorAbierto, selectorRef]);

  useImperativeHandle(ref, () => ({
    filasFiltradas: () => {
      const resultado: T[] = [];
      apiRef.current?.forEachNodeAfterFilter((nodo) => {
        if (nodo.data) resultado.push(nodo.data);
      });
      return resultado;
    },
    columnasVisibles: () =>
      (apiRef.current?.getColumnState() ?? [])
        .filter((columna) => !columna.hide)
        .map((columna) => columna.colId),
    exportarCsv,
  }));

  const conFiltro = filtrosPorColumna === true && filtrosVisibles;

  // `claseFila` puede depender del reloj (ej. "más de 12 horas adentro"):
  // sin esto, una fila que cruza el umbral no cambiaba hasta el próximo
  // refresco de datos.
  useEffect(() => {
    if (!claseFila) return;
    const intervalo = window.setInterval(() => apiRef.current?.redrawRows(), 60_000);
    return () => window.clearInterval(intervalo);
  }, [claseFila]);

  const columnasConVisibilidad = useMemo(
    () =>
      columnas.map((original) => {
        const clave = identidad(original as ColDef<unknown>);
        // `type` se resuelve acá (no con `columnTypes` de AG Grid) para
        // que dependa de si los filtros están visibles.
        const { type, ...resto } = original;
        let columna: ColDef<T> = clave ? { ...resto, hide: ocultas.has(clave) } : resto;
        if (type === "fecha" && conFiltro) columna = { ...columna, ...(FILTRO_FECHA as ColDef<T>) };
        if (type === "numero" && conFiltro) columna = { ...columna, ...(FILTRO_NUMERO as ColDef<T>) };
        return columna;
      }),
    [columnas, ocultas, conFiltro],
  );

  const columnaBase = useMemo<ColDef>(
    () => ({
      ...(conFiltro ? columnaPorDefectoConFiltro : columnaPorDefecto),
      ...(idFila ? { enableCellChangeFlash: true } : {}),
    }),
    [conFiltro, idFila],
  );

  async function exportarCsv(nombre: string | undefined = nombreExportacion) {
    const api = apiRef.current;
    if (!api || !nombre) return;
    // Sólo columnas con dato (no las de botones ni la de casillas).
    const columnKeys = api
      .getAllDisplayedColumns()
      .filter((columna) => {
        const definicion = columna.getColDef();
        return definicion.field !== undefined || typeof definicion.valueGetter === "function";
      })
      .map((columna) => columna.getColId());
    if (columnKeys.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }
    // Punto y coma: Excel en español (Costa Rica usa coma decimal) espera
    // ese separador; con coma abriría todo en una sola columna.
    const contenido = api.getDataAsCsv({
      columnKeys,
      columnSeparator: ";",
    });
    if (!contenido) {
      toast.error("No hay filas para exportar.");
      return;
    }
    const destino = await save({
      title: "Exportar a CSV",
      defaultPath: `${nombre}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!destino) return;
    try {
      await guardarCsv(destino, contenido);
      toast.success("CSV exportado.");
    } catch (error) {
      toast.error(String(error));
    }
  }

  /** Cada columna al ancho de su contenido. Se les saca el `flex` (reparto
   * proporcional del espacio) porque si no, AG Grid lo vuelve a aplicar
   * encima al primer cambio de tamaño. Queda guardado como cualquier otro
   * cambio de ancho; "Restablecer anchos" vuelve al reparto original. */
  function ajustarAnchos() {
    const api = apiRef.current;
    if (!api) return;
    api.applyColumnState({
      state: api.getColumnState().map((columna) => ({ colId: columna.colId, flex: null })),
    });
    api.autoSizeAllColumns();
    setSelectorAbierto(false);
  }

  function restablecerAnchos() {
    const api = apiRef.current;
    if (!api) return;
    api.applyColumnState({
      state: columnas
        .map((columna) => ({
          colId: identidad(columna as ColDef<unknown>),
          flex: columna.flex ?? null,
          width: columna.flex ? undefined : columna.width,
        }))
        .filter((estado): estado is { colId: string; flex: number | null; width: number | undefined } =>
          estado.colId !== undefined,
        ),
    });
    guardarLayout(ocultas);
    setSelectorAbierto(false);
  }

  function alternar(clave: string) {
    setOcultas((actual) => {
      const siguiente = new Set(actual);
      if (siguiente.has(clave)) {
        siguiente.delete(clave);
      } else {
        siguiente.add(clave);
      }
      guardarLayout(siguiente);
      return siguiente;
    });
  }

  /** Guarda ocultas + el estado de columnas (orden, ancho, sort, pin) tal
   * cual lo tiene la grilla en este momento — se llama tanto al tocar el
   * selector como al mover/redimensionar/ordenar una columna. `hide` se
   * excluye del estado de AG Grid a propósito: `ocultas` ya es la única
   * fuente de verdad para visibilidad (ver `columnasConVisibilidad`); si el
   * estado de AG Grid trajera su propio `hide`, ambas fuentes podrían
   * contradecirse. */
  function guardarLayout(ocultasActual: Set<string>, filtrosVisiblesActual: boolean = filtrosVisibles) {
    if (!idGrilla || !apiRef.current) return;
    const columnState = apiRef.current.getColumnState().map(({ hide: _hide, ...resto }) => resto);
    const estado: EstadoGuardado = {
      ocultas: Array.from(ocultasActual),
      columnas: columnState,
      filtrosVisibles: filtrosVisiblesActual,
    };
    try {
      localStorage.setItem(claveAlmacenamiento(idGrilla), JSON.stringify(estado));
    } catch {
      // localStorage puede fallar (modo privado, cuota llena) — perder el
      // layout guardado no es motivo para romper la grilla.
    }
  }

  function alternarFiltrosVisibles() {
    setFiltrosVisibles((actual) => {
      const siguiente = !actual;
      guardarLayout(ocultas, siguiente);
      return siguiente;
    });
  }

  function alListo(evento: GridReadyEvent<T>) {
    apiRef.current = evento.api;
    const guardado = leerEstadoGuardado(idGrilla);
    if (guardado?.columnas?.length) {
      evento.api.applyColumnState({ state: guardado.columnas, applyOrder: true });
    }
  }

  // La app mantiene TODAS las secciones ya visitadas montadas para siempre
  // (ocultas con CSS `display: none`, ver el doc-comment de `visitadas` en
  // App.tsx) -- `onGridReady` sólo corre una vez, al montar, pero AG Grid
  // recalcula el ancho de las columnas `flex` cada vez que su contenedor se
  // remide, y eso incluye pasar de `display: none` a visible otra vez al
  // volver a esta sección. Esa recalculación pisa el layout ya aplicado.
  // Reaplicarlo cada vez que la sección vuelve a activarse (no sólo al
  // montar) es lo que evita que un usuario pierda el orden/ancho que ya
  // había acomodado con sólo cambiar de pestaña y volver.
  const seccionActiva = useSeccionActiva();
  useEffect(() => {
    if (!seccionActiva || !apiRef.current) return;
    const guardado = leerEstadoGuardado(idGrilla);
    if (guardado?.columnas?.length) {
      apiRef.current.applyColumnState({ state: guardado.columnas, applyOrder: true });
    }
  }, [seccionActiva, idGrilla]);

  function alMoverColumna(evento: ColumnMovedEvent<T>) {
    if (evento.finished) guardarLayout(ocultas);
  }

  function alRedimensionarColumna(evento: ColumnResizedEvent<T>) {
    if (evento.finished) guardarLayout(ocultas);
  }

  function alOrdenar(_evento: SortChangedEvent<T>) {
    guardarLayout(ocultas);
  }

  function alFijarColumna(_evento: ColumnPinnedEvent<T>) {
    guardarLayout(ocultas);
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          display: "flex",
          alignItems: "flex-end",
          justifyContent: "space-between",
          gap: "0.375rem",
          marginBottom: "0.375rem",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "flex-end",
            gap: "0.375rem",
            flexWrap: "wrap",
            flex: 1,
          }}
        >
          {controles}
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.375rem" }}>
          {accionesDerecha}

          {nombreExportacion && (
            // Mismo botón de ícono que Excel/PDF en Historial.
            <button
              type="button"
              className="boton boton-icono"
              title="Exportar a CSV — respeta el filtro/orden/columnas actuales de la grilla"
              onClick={() => exportarCsv()}
            >
              <Sheet size={16} />
            </button>
          )}

          <div ref={selectorRef}>
            <button type="button" className="boton" onClick={() => setSelectorAbierto((a) => !a)}>
              Columnas ▾
            </button>
          </div>

          {selectorAbierto && posicionSelector && (
            <ListaFlotante posicion={posicionSelector} ancho={220} alinear="derecha">
              <div
                ref={popoverRef}
                style={{
                  padding: "0.75rem 1rem",
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.4rem",
                }}
              >
                {columnas
                  .map((columna) => ({ columna, clave: identidad(columna as ColDef<unknown>) }))
                  .filter(
                    (entrada): entrada is { columna: ColDef<T>; clave: string } =>
                      entrada.clave !== undefined,
                  )
                  .map(({ columna, clave }) => (
                    <label
                      key={clave}
                      style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
                    >
                      <input
                        type="checkbox"
                        checked={!ocultas.has(clave)}
                        onChange={() => alternar(clave)}
                      />
                      {columna.headerName ?? clave}
                    </label>
                  ))}
                {filtrosPorColumna && (
                  <>
                    <hr style={{ width: "100%", border: "none", borderTop: "1px solid var(--borde)", margin: "0.2rem 0" }} />
                    <label style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                      <input
                        type="checkbox"
                        checked={filtrosVisibles}
                        onChange={alternarFiltrosVisibles}
                      />
                      Filtros por columna
                    </label>
                  </>
                )}
                <hr style={{ width: "100%", border: "none", borderTop: "1px solid var(--borde)", margin: "0.2rem 0" }} />
                <button type="button" className="boton" onClick={ajustarAnchos}>
                  Ajustar anchos al contenido
                </button>
                <button type="button" className="boton" onClick={restablecerAnchos}>
                  Restablecer anchos
                </button>
              </div>
            </ListaFlotante>
          )}
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0 }}>
        <AgGridReact<T>
          theme={temaBrisas}
          defaultColDef={columnaBase}
          rowData={filas}
          getRowId={
            idFila
              ? (p: GetRowIdParams<T>) => idFila(p.data)
              : undefined
          }
          getRowClass={
            claseFila
              ? (p: RowClassParams<T>) =>
                  (p.data ? claseFila(p.data) : undefined)
              : undefined
          }
          // Sin el recuadro de foco al hacer clic en una celda (se veía
          // feo y no copiaba nada) -- a cambio no hay navegación por
          // flechas dentro de la grilla.
          suppressCellFocus
          columnHoverHighlight
          // "Álvarez" junto a las A, no al final de la lista.
          accentedSort
          columnDefs={columnasConVisibilidad}
          quickFilterText={busqueda}
          quickFilterParser={quickFilterParser}
          quickFilterMatcher={quickFilterMatcher}
          overlayNoRowsTemplate={MENSAJE_SIN_FILAS}
          // Menús de filtro, "Cargando…", etc. en español -- sin esto AG
          // Grid mostraba "Contains", "Equals", "AND/OR" en inglés.
          localeText={AG_GRID_LOCALE_ES}
          loading={cargando === true && filas.length === 0}
          tooltipShowMode="whenTruncated"
          tooltipShowDelay={400}
          // Resguardo además de memoizar `columnas` en cada pantalla: si de
          // todos modos algo le pasa un `columnDefs` nuevo, esto evita que
          // AG Grid reordene según el orden literal del array en vez de
          // conservar el que el usuario ya acomodó a mano.
          maintainColumnOrder
          rowHeight={36}
          headerHeight={38}
          rowSelection={
            seleccionMultiple
              ? { mode: "multiRow", checkboxes: true, headerCheckbox: true }
              : undefined
          }
          onGridReady={alListo}
          onColumnMoved={alMoverColumna}
          onColumnResized={alRedimensionarColumna}
          onSortChanged={alOrdenar}
          onColumnPinned={alFijarColumna}
          onSelectionChanged={
            onSeleccionCambia
              ? (evento) => onSeleccionCambia(evento.api.getSelectedRows())
              : undefined
          }
          onCellValueChanged={
            onCeldaEditada ? (evento) => onCeldaEditada(evento.data) : undefined
          }
          // Selección múltiple: un clic en cualquier parte de la fila la
          // marca/desmarca, igual que la casilla (sin afectar a las demás).
          // A mano en vez de `enableClickSelection` de AG Grid porque ése
          // también se disparaba al tocar el botón "Salida" de la fila.
          onRowClicked={
            seleccionMultiple
              ? (evento: RowClickedEvent<T>) => {
                  if (clicEnControlInteractivo(evento.event?.target)) return;
                  evento.node.setSelected(!evento.node.isSelected());
                }
              : undefined
          }
          onRowDoubleClicked={
            onFilaDobleClic ? (evento) => evento.data && onFilaDobleClic(evento.data) : undefined
          }
        />
      </div>
    </div>
  );
}

// `forwardRef` no admite tipos genéricos por su cuenta — el `as` restaura la
// firma genérica de `TablaBase` para quien use `<Tabla<T> ref={...} />`.
const Tabla = forwardRef(TablaBase) as <T>(
  props: TablaProps<T> & { ref?: React.ForwardedRef<TablaHandle<T>> },
) => ReturnType<typeof TablaBase>;

export default Tabla;
