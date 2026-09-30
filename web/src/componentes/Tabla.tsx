import "./modulosTabla";
import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AgGridReact } from "ag-grid-react";
import { themeQuartz } from "ag-grid-community";
import { AG_GRID_LOCALE_ES } from "@ag-grid-community/locale";
import { Columns3, Funnel, RotateCcw, UnfoldHorizontal } from "lucide-react";
import type {
  ColDef,
  ColumnMovedEvent,
  ColumnPinnedEvent,
  ColumnResizedEvent,
  ColumnState,
  GetRowIdParams,
  GridReadyEvent,
  IDatasource,
  IGetRowsParams,
  SortChangedEvent,
  TextMatcherParams,
} from "ag-grid-community";
import { useUsuarioId } from "../contexto/SesionContexto";
import { ListaFlotante, useListaFlotante } from "./ListaFlotante";
import FiltroFechaTabla from "./FiltroFechaTabla";
import { compararFechaYMD, textoTooltip } from "./Tabla.logica";
import { useDebounced } from "./useDebounced";

/**
 * Tema y comportamiento compartido de TODAS las tablas de la app — un solo
 * punto para cambiar cómo se ven/comportan las grillas. Usa las mismas
 * custom properties que el resto de la app (index.css) en vez de colores
 * fijos, para que la grilla siga el tema claro/oscuro del sistema en lugar
 * de quedar siempre oscura sin importar el resto de la interfaz. Copiado de
 * `desktop/src/componentes/Tabla.tsx`.
 *
 * Deliberadamente NO expone cualquier prop de AgGridReact. Mostrar/ocultar
 * columnas es capacidad permanente (no un flag opcional). Selección múltiple
 * sí es opt-in (`seleccionMultiple`), porque no toda pantalla la necesita.
 */
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

/** Sin tildes y en mayúsculas -- mismo criterio de "sin tildes/mayúsculas no
 * importa" que el buscador del núcleo. El `quickFilterText` propio de AG Grid
 * sólo hace `.toUpperCase()`, sin tocar diacríticos: buscar "Sanches" no
 * encontraba a "Sánchez". */
function plegar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toUpperCase();
}

/** `quickFilterParser`/`quickFilterMatcher`: el parser pliega cada palabra
 * tecleada, el matcher pliega el texto agregado de la fila antes de comparar.
 * Mismo AND implícito que el default de AG Grid: todas las palabras tienen
 * que aparecer en algún lado de la fila. */
function quickFilterParser(texto: string): string[] {
  return plegar(texto)
    .split(" ")
    .filter((parte) => parte.length > 0);
}

function quickFilterMatcher(partes: string[], textoFila: string): boolean {
  const plegado = plegar(textoFila);
  return partes.every((parte) => plegado.includes(parte));
}

/** Lo mismo que el quick filter, pero en el filtro POR COLUMNA de texto (código
 * aparte dentro de AG Grid): sin tildes y, en "contiene", por palabras. Sólo
 * cuenta en las tablas que filtran en el navegador; las de servidor filtran en
 * la base con las mismas reglas (`api/historialFiltros.ts`). */
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

const columnaPorDefecto: ColDef = {
  sortable: true,
  resizable: true,
  minWidth: 90,
  // Centrado por defecto (encabezado y dato) en todas las grillas — la
  // columna de nombre es la excepción explícita, cada pantalla la anula
  // con `cellStyle: { textAlign: "left" }` (el encabezado se queda
  // centrado igual, sólo el dato cambia).
  headerClass: "columna-centrada",
  cellStyle: { textAlign: "center" },
  // Texto completo al pasar el mouse, pero sólo en celdas cortadas
  // ("KAREN DE LOS ANGELE…") -- ver `tooltipShowMode="whenTruncated"`.
  tooltipValueGetter: textoTooltip,
};

const MENSAJE_SIN_FILAS = `<span style="color: var(--muted); font-size: 0.9rem;">Sin resultados</span>`;

/** `type: "fecha"` / `type: "numero"` en la definición de una columna: filtro de
 * fecha (antes, después, entre...) o de número (mayor que, menor que...) en vez
 * del de texto. Sólo aplica con los filtros por columna visibles. */
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

// v2: el layout guardado incluye `pinned` por columna.
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
 * explícito (ej. dos columnas que leen el mismo `field`), si no el `field`.
 * Mismo criterio que usa AG Grid internamente para su propio
 * `getColumnState`. */
export function identidad(columna: ColDef<unknown>): string | undefined {
  if (typeof columna.colId === "string") return columna.colId;
  if (typeof columna.field === "string") return columna.field;
  return undefined;
}

/** Orden elegido en la grilla (una sola columna). */
export interface OrdenTabla {
  colId: string;
  descendente: boolean;
}

/** Lo que la grilla le pide al servidor: un tramo de filas con su orden y sus
 * filtros por columna (el `getFilterModel()` de AG Grid). */
export interface PeticionPagina {
  inicio: number;
  cantidad: number;
  orden: OrdenTabla | null;
  filtros: Record<string, unknown>;
}

/** Tabla paginada en el servidor (modelo de filas infinito de AG Grid): la
 * grilla no filtra ni ordena filas en el navegador, sino que pide cada página
 * ya filtrada y ordenada. */
export interface OrigenServidor<T> {
  tamanoPagina: number;
  cargarPagina: (peticion: PeticionPagina) => Promise<{ filas: T[]; total: number }>;
  /** Una página no se pudo traer (la grilla queda con esa página vacía). */
  alFallar: (error: unknown) => void;
}

export interface TablaProps<T> {
  columnas: ColDef<T>[];
  /** Filas que la tabla tiene cargadas. No se usa con `origenServidor`. */
  filas?: T[];
  /** Controles propios de la pantalla (ej. buscador, "+ Nuevo…") — se
   * muestran en la misma línea que el grupo de íconos de columnas, a la
   * izquierda. */
  controles?: ReactNode;
  /** Igual que `controles`, pero a la derecha, junto al grupo de columnas. */
  accionesDerecha?: ReactNode;
  /** Texto de búsqueda global (una sola caja, busca en todas las columnas)
   * — alternativa a `filtrosPorColumna` para listas donde un filtro por
   * columna es más de lo que hace falta. Sólo en tablas que filtran en el
   * navegador. */
  busqueda?: string;
  /** Checkbox por fila + checkbox de encabezado para seleccionar varias a la
   * vez. */
  seleccionMultiple?: boolean;
  onSeleccionCambia?: (filas: T[]) => void;
  /** Se dispara cuando el usuario edita una celda editable (ej. un checkbox
   * de columna booleana) — entrega la fila completa ya actualizada para que
   * la pantalla decida cómo persistirla. */
  onCeldaEditada?: (fila: T) => void;
  /** Doble click en una fila — pensado para abrir edición. */
  onFilaDobleClic?: (fila: T) => void;
  /** Filtro por columna (fila de filtros bajo el encabezado) en vez del
   * `controles` propio de la pantalla. */
  filtrosPorColumna?: boolean;
  /** Identidad estable de cada fila (obligatoria en la práctica con
   * `origenServidor`: al refrescar, AG Grid reconoce la misma fila). */
  idFila?: (fila: T) => string;
  /** Paginación y filtros en el servidor, ver `OrigenServidor`. */
  origenServidor?: OrigenServidor<T>;
  /** Identificador estable de esta grilla. Habilita persistir en
   * localStorage el orden, ancho, orden de columnas (sort) y cuáles están
   * ocultas. */
  id?: string;
}

/** Mango imperativo opcional (`ref`) para que la pantalla pida datos que
 * viven adentro de la grilla sin tener que duplicar su estado — hoy "las
 * filas que quedaron visibles tras el filtro por columna", "qué columnas
 * están visibles ahora" y, en tablas de servidor, el filtro y el orden
 * vigentes para exportar exactamente lo que se ve. */
export interface TablaHandle<T> {
  filasFiltradas: () => T[];
  /** Identidades (`colId`/`field`) de las columnas visibles ahora mismo, en
   * el orden real de la grilla. */
  columnasVisibles: () => string[];
  /** Tablas de servidor: vuelve a pedir la página actual (ej. llegó un cambio
   * por Realtime), o desde la primera con `desdeElInicio` (ej. cambió el
   * rango de fechas o la búsqueda de la pantalla). */
  refrescar: (desdeElInicio?: boolean) => void;
  /** Modelo de filtros por columna vigente (`getFilterModel()`). */
  modeloFiltros: () => Record<string, unknown>;
  /** Columna y dirección del orden vigente, o `null`. */
  orden: () => OrdenTabla | null;
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
    idFila,
    origenServidor,
    id,
  }: TablaProps<T>,
  ref: React.ForwardedRef<TablaHandle<T>>,
) {
  const usuarioId = useUsuarioId();
  const idGrilla = idPorUsuario(id, usuarioId);
  // AG Grid recalcula el quickFilter sobre TODAS las filas cargadas
  // (client-side, ver el doc-comment de `busqueda` arriba) en cada
  // pulsación -- con un dataset grande eso es trabajo real por tecla. El
  // input en sí (lo que la persona ve mientras escribe) no se debounce --
  // sólo lo que le llega a AG Grid.
  const busquedaDebounced = useDebounced(busqueda, 250);
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
  const conFiltro = filtrosPorColumna === true && filtrosVisibles;
  const enServidor = origenServidor !== undefined;

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

  function ordenVigente(): OrdenTabla | null {
    const ordenada = (apiRef.current?.getColumnState() ?? [])
      .filter((columna) => columna.sort)
      .sort((a, b) => (a.sortIndex ?? 0) - (b.sortIndex ?? 0))[0];
    return ordenada ? { colId: ordenada.colId, descendente: ordenada.sort === "desc" } : null;
  }

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
    refrescar: (desdeElInicio) => {
      const api = apiRef.current;
      if (!api) return;
      if (desdeElInicio) {
        api.paginationGoToFirstPage();
        api.purgeInfiniteCache();
      } else {
        api.refreshInfiniteCache();
      }
    },
    modeloFiltros: () => (apiRef.current?.getFilterModel() ?? {}) as Record<string, unknown>,
    orden: ordenVigente,
  }));

  const columnasConVisibilidad = useMemo(
    () =>
      columnas.map((original) => {
        const clave = identidad(original as ColDef<unknown>);
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

  // El origen cambia de identidad en cada render de la pantalla; la grilla
  // mantiene un solo `datasource` y siempre llama al más reciente.
  const origenRef = useRef(origenServidor);
  useEffect(() => {
    origenRef.current = origenServidor;
  });
  const tamanoPagina = origenServidor?.tamanoPagina;
  const datasource = useMemo<IDatasource | undefined>(() => {
    if (tamanoPagina === undefined) return undefined;
    return {
      getRows(params: IGetRowsParams) {
        const origen = origenRef.current;
        if (!origen) {
          params.failCallback();
          return;
        }
        const primeraOrden = params.sortModel[0];
        origen
          .cargarPagina({
            inicio: params.startRow,
            cantidad: params.endRow - params.startRow,
            orden: primeraOrden ? { colId: primeraOrden.colId, descendente: primeraOrden.sort === "desc" } : null,
            filtros: params.filterModel as Record<string, unknown>,
          })
          .then(({ filas: pagina, total }) => params.successCallback(pagina, total))
          .catch((error: unknown) => {
            origen.alFallar(error);
            params.failCallback();
          });
      },
    };
  }, [tamanoPagina]);

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
   * cual lo tiene la grilla en este momento. `hide` se excluye del estado de
   * AG Grid a propósito: `ocultas` ya es la única fuente de verdad para
   * visibilidad. */
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
  }

  function alListo(evento: GridReadyEvent<T>) {
    apiRef.current = evento.api;
    const guardado = leerEstadoGuardado(idGrilla);
    if (guardado?.columnas?.length) {
      evento.api.applyColumnState({ state: guardado.columnas, applyOrder: true });
    }
  }

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
    <div className="flex h-full flex-col">
      <div
        className="mb-[0.375rem] flex flex-wrap items-end justify-between gap-[0.375rem]"
      >
        <div
          className="flex flex-1 flex-wrap items-end gap-[0.375rem]"
        >
          {controles}
        </div>

        <div className="flex items-center gap-[0.375rem]">
          {accionesDerecha}

          {/* Filtros y anchos de columna en una sola pieza de íconos
              (`.segmentado`), como en escritorio. El embudo es un interruptor
              (relleno de acento con los filtros visibles); los otros dos son
              acciones, y el último abre la lista de columnas visibles. */}
          <div
            ref={selectorRef}
            className="segmentado"
            role="group"
            aria-label="Columnas: filtros, anchos y visibles"
          >
            {filtrosPorColumna && (
              <button
                type="button"
                className="segmentado-interruptor"
                title={filtrosVisibles ? "Ocultar filtros" : "Mostrar filtros"}
                aria-label="Filtros por columna"
                aria-pressed={filtrosVisibles}
                onClick={alternarFiltrosVisibles}
              >
                <Funnel size={16} aria-hidden="true" />
              </button>
            )}
            <button
              type="button"
              title="Ajustar anchos al contenido"
              aria-label="Ajustar anchos al contenido"
              onClick={ajustarAnchos}
            >
              <UnfoldHorizontal size={16} aria-hidden="true" />
            </button>
            <button
              type="button"
              title="Restablecer anchos"
              aria-label="Restablecer anchos"
              onClick={restablecerAnchos}
            >
              <RotateCcw size={16} aria-hidden="true" />
            </button>
            <button
              type="button"
              title="Columnas visibles"
              aria-label="Columnas visibles"
              aria-expanded={selectorAbierto}
              aria-pressed={selectorAbierto}
              className="segmentado-interruptor"
              onClick={() => setSelectorAbierto((a) => !a)}
            >
              <Columns3 size={16} aria-hidden="true" />
            </button>
          </div>

          {selectorAbierto && posicionSelector && (
            <ListaFlotante posicion={posicionSelector} ancho={220} alinear="derecha">
              <div
                ref={popoverRef}
                className="flex flex-col gap-[0.4rem] px-4 py-3"
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
                      className="flex items-center gap-2"
                    >
                      <input
                        type="checkbox"
                        checked={!ocultas.has(clave)}
                        onChange={() => alternar(clave)}
                      />
                      {columna.headerName ?? clave}
                    </label>
                  ))}
              </div>
            </ListaFlotante>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1">
        <AgGridReact<T>
          theme={temaBrisas}
          defaultColDef={columnaBase}
          {...(enServidor
            ? {
                rowModelType: "infinite" as const,
                datasource,
                cacheBlockSize: tamanoPagina,
                pagination: true,
                paginationPageSize: tamanoPagina,
                paginationPageSizeSelector: false,
                maxConcurrentDatasourceRequests: 1,
              }
            : {
                rowData: filas ?? [],
                quickFilterText: busquedaDebounced,
                quickFilterParser,
                quickFilterMatcher,
                // "Álvarez" junto a las A, no al final de la lista.
                accentedSort: true,
              })}
          getRowId={idFila ? (p: GetRowIdParams<T>) => idFila(p.data) : undefined}
          columnDefs={columnasConVisibilidad}
          overlayNoRowsTemplate={MENSAJE_SIN_FILAS}
          localeText={AG_GRID_LOCALE_ES}
          // Sin el recuadro de foco al hacer clic en una celda (se veía feo y
          // no copiaba nada) -- a cambio no hay navegación por flechas dentro
          // de la grilla.
          suppressCellFocus
          columnHoverHighlight
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
