import { forwardRef, useImperativeHandle, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AgGridReact } from "ag-grid-react";
import { themeQuartz } from "ag-grid-community";
import type {
  ColDef,
  ColumnMovedEvent,
  ColumnPinnedEvent,
  ColumnResizedEvent,
  ColumnState,
  GridReadyEvent,
  SortChangedEvent,
} from "ag-grid-community";

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
const temaBrisas = themeQuartz.withParams({
  backgroundColor: "var(--panel)",
  foregroundColor: "var(--texto)",
  headerBackgroundColor: "var(--panel-suave)",
  headerTextColor: "var(--muted)",
  borderColor: "var(--borde)",
  accentColor: "var(--acento)",
  oddRowBackgroundColor: "var(--campo-fondo)",
  borderRadius: 8,
  wrapperBorderRadius: 8,
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
};

const MENSAJE_SIN_FILAS = `<span style="color: var(--muted); font-size: 0.9rem;">Sin resultados</span>`;

const columnaPorDefectoConFiltro: ColDef = {
  ...columnaPorDefecto,
  filter: true,
  floatingFilter: true,
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
  }: TablaProps<T>,
  ref: React.ForwardedRef<TablaHandle<T>>,
) {
  const [ocultas, setOcultas] = useState<Set<string>>(
    () => new Set(leerEstadoGuardado(id)?.ocultas ?? []),
  );
  const [filtrosVisibles, setFiltrosVisibles] = useState(
    () => leerEstadoGuardado(id)?.filtrosVisibles ?? true,
  );
  const [selectorAbierto, setSelectorAbierto] = useState(false);
  const apiRef = useRef<GridReadyEvent<T>["api"] | null>(null);

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
  }));

  const columnasConVisibilidad = useMemo(
    () =>
      columnas.map((columna) => {
        const clave = identidad(columna as ColDef<unknown>);
        return clave ? { ...columna, hide: ocultas.has(clave) } : columna;
      }),
    [columnas, ocultas],
  );

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
    if (!id || !apiRef.current) return;
    const columnState = apiRef.current.getColumnState().map(({ hide: _hide, ...resto }) => resto);
    const estado: EstadoGuardado = {
      ocultas: Array.from(ocultasActual),
      columnas: columnState,
      filtrosVisibles: filtrosVisiblesActual,
    };
    try {
      localStorage.setItem(claveAlmacenamiento(id), JSON.stringify(estado));
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
    const guardado = leerEstadoGuardado(id);
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          display: "flex",
          alignItems: "flex-end",
          justifyContent: "space-between",
          gap: "0.75rem",
          marginBottom: "0.5rem",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "flex-end",
            gap: "0.75rem",
            flexWrap: "wrap",
            flex: 1,
          }}
        >
          {controles}
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
          {accionesDerecha}

          <div style={{ position: "relative" }}>
            <button type="button" className="boton" onClick={() => setSelectorAbierto((a) => !a)}>
              Columnas ▾
            </button>

            {selectorAbierto && (
              <>
                {/* Backdrop invisible: cierra el selector al hacer click afuera. */}
                <div
                  onClick={() => setSelectorAbierto(false)}
                  style={{ position: "fixed", inset: 0, zIndex: 9 }}
                />
                <div
                  className="tarjeta"
                  style={{
                    position: "absolute",
                    top: "2.35rem",
                    right: 0,
                    zIndex: 10,
                    padding: "0.75rem 1rem",
                    display: "flex",
                    flexDirection: "column",
                    gap: "0.4rem",
                    minWidth: "13rem",
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
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0 }}>
        <AgGridReact<T>
          theme={temaBrisas}
          defaultColDef={
            filtrosPorColumna && filtrosVisibles ? columnaPorDefectoConFiltro : columnaPorDefecto
          }
          rowData={filas}
          columnDefs={columnasConVisibilidad}
          quickFilterText={busqueda}
          overlayNoRowsTemplate={MENSAJE_SIN_FILAS}
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
