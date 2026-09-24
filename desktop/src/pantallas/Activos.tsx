import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { CircleOff, IdCard, Users } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import Modal from "../componentes/Modal";
import { useAccionBarraEstado, useBarraEstado } from "../contexto/BarraEstadoContexto";
import { cerrarFilaActiva, claveFilaActiva, listarTodosLosActivos, textoMedioConPlaca } from "../api";
import type { FilaActiva } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/** Filtro rápido por gafete (pedido del usuario 2026-09-23): ver sólo a
 * quienes entraron sin gafete ("S/G", `gafete_numero` nulo), sólo a
 * quienes tienen uno, o a todos. */
export type FiltroGafete = "todos" | "con" | "sin";

const ETIQUETAS_FILTRO_GAFETE: Record<FiltroGafete, string> = {
  todos: "Todos",
  con: "Con gafete",
  sin: "S/G",
};

/** Ícono y nombre completo (al pasar el mouse) de cada opción -- el
 * control muestra sólo íconos (pedido del usuario 2026-09-23). */
const OPCIONES_FILTRO_GAFETE: OpcionSegmentada<FiltroGafete>[] = [
  { valor: "todos", Icono: Users, titulo: "Todos" },
  { valor: "con", Icono: IdCard, titulo: "Con gafete" },
  { valor: "sin", Icono: CircleOff, titulo: "Sin gafete (S/G)" },
];

export function filtrarPorGafete<T extends { gafete_numero: number | null }>(
  filas: readonly T[],
  filtro: FiltroGafete,
): T[] {
  if (filtro === "todos") return [...filas];
  const sinGafete = filtro === "sin";
  return filas.filter((fila) => (fila.gafete_numero == null) === sinGafete);
}

/** Pieza única de íconos con el relleno deslizante (`SegmentadoOpciones`). */
function ToggleGafete({
  filtro,
  onCambiar,
}: {
  filtro: FiltroGafete;
  onCambiar: (filtro: FiltroGafete) => void;
}) {
  return (
    <SegmentadoOpciones
      opciones={OPCIONES_FILTRO_GAFETE}
      valor={filtro}
      onCambiar={onCambiar}
      etiqueta="Filtrar por gafete"
    />
  );
}

const DOCE_HORAS_MS = 12 * 60 * 60 * 1000;

/** Más de 12 horas adentro desde el ingreso -- único resaltado de filas
 * que pidió el usuario (2026-09-23). `ahora` inyectable para el test. */
export function masDeDoceHoras(
  fila: { fecha_hora_ingreso: string },
  ahora: number = Date.now(),
): boolean {
  return ahora - new Date(fila.fecha_hora_ingreso).getTime() > DOCE_HORAS_MS;
}

function claseFilaActiva(fila: FilaActiva): string | undefined {
  return masDeDoceHoras(fila) ? "fila-mas-de-12-horas" : undefined;
}

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirSalida,
}: {
  /** Los modales de Nuevo Ingreso y Salida viven en el Shell (se disparan
   * desde cualquier pantalla vía Ctrl+N/S, no sólo desde acá) —
   * este número sube cada vez que registran algo, para que la grilla se
   * refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
  onAbrirSalida: () => void;
}) {
  const [filas, setFilas] = useState<FilaActiva[]>([]);
  const [total, setTotal] = useState(0);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const [seleccionadas, setSeleccionadas] = useState<FilaActiva[]>([]);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);
  const [filtroGafete, setFiltroGafete] = useState<FiltroGafete>("todos");
  const filasVisibles = useMemo(() => filtrarPorGafete(filas, filtroGafete), [filas, filtroGafete]);
  const sinGafete = useMemo(() => filtrarPorGafete(filas, "sin").length, [filas]);

  useBarraEstado(
    cargando
      ? "Cargando…"
      : filtroGafete === "todos"
        ? `${total} adentro · ${sinGafete} S/G`
        : `${filasVisibles.length} de ${total} adentro (${ETIQUETAS_FILTRO_GAFETE[filtroGafete]})`,
  );

  // En la barra de estado, no sobre la grilla: antes una franja con
  // "N seleccionado(s)" aparecía arriba y corría toda la pantalla hacia
  // abajo (pedido del usuario 2026-09-23).
  useAccionBarraEstado(
    seleccionadas.length > 0 ? `Registrar salida (${seleccionadas.length})` : null,
    () => setConfirmarSalidaMasiva(true),
  );

  const recargar = useCallback(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- de lo contrario `react-hooks/set-state-in-effect` marca
    // esta actualización de estado como síncrona dentro del cuerpo del
    // efecto que la dispara (abajo). Diferirla a un microtask no cambia
    // nada perceptible (corre antes del próximo paint igual) y cumple la
    // regla.
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarTodosLosActivos())
      .then(({ filas, total }) => {
        setFilas(filas);
        setTotal(total);
        setSeleccionadas([]);
      })
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar, refrescarSenal]);

  // useCallback a propósito: esta función se cierra dentro de una celda de
  // `columnas` — sin identidad estable, `columnas` (memoizado más abajo)
  // se recrearía en cada render igual, y con eso AG Grid reasignaría el
  // orden/ancho originales del código encima de lo que el usuario acomodó
  // (ver el comentario junto a `columnas`).
  const salidaIndividual = useCallback(
    async (fila: FilaActiva) => {
      try {
        await cerrarFilaActiva(fila);
        recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    try {
      for (const fila of seleccionadas) {
        await cerrarFilaActiva(fila);
      }
      setConfirmarSalidaMasiva(false);
      await recargar();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setProcesando(false);
    }
  }

  // useMemo a propósito: si `columnas` se recrea en cada render (ej. cada
  // vez que `recargar()` trae datos nuevos), AG Grid recibe un `columnDefs`
  // "nuevo" y reaplica el orden/ancho literales de acá encima de lo que el
  // usuario ya había acomodado a mano — el layout persistido en
  // `Tabla`/localStorage quedaba pisado en el siguiente refresco.
  const columnas: ColDef<FilaActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.2, minWidth: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.1, minWidth: 140 },
      {
        field: "tipo_ingreso",
        headerName: "Tipo",
        flex: 1,
        minWidth: 100,
        // Buscador de arriba (quickFilter) limitado a Cédula/Nombre/Empresa
        // -- las tres cosas que identifican a LA PERSONA que se busca.
        // `getQuickFilterText: () => ""` saca esta columna de esa búsqueda
        // sin afectar el filtro de columna propio (`floatingFilter`), que
        // sigue funcionando normal. Mismo criterio en el resto de columnas
        // de abajo -- hallazgo real del usuario 2026-09-21: buscar "daniel"
        // (un contratista) traía en cambio el registro dado de alta por el
        // operador Daniel, porque el buscador también miraba "Dio ingreso".
        getQuickFilterText: () => "",
      },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueFormatter: (p) =>
          p.value == null ? "—" : textoMedioConPlaca(p.value, p.data?.placa ?? null),
        getQuickFilterText: () => "",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
        getQuickFilterText: () => "",
      },
      {
        colId: "fecha_ingreso",
        type: "fecha",
        headerName: "Fecha",
        flex: 1.1,
        minWidth: 110,
        // `valueGetter` (no `field`) a propósito, igual que Hora: si la
        // columna lee el string ISO crudo, AG Grid la infiere como fecha y
        // el filtro flotante termina siendo el selector nativo de
        // fecha+hora del navegador (`datetime-local`) en vez de un texto
        // simple — igual que le pasaba a Hora. `fechaLocalYMD` mantiene el
        // orden cronológico correcto como texto ("2026-08-28" ordena bien
        // sin volver a pasar por `Date`); `valueFormatter` sólo reacomoda
        // ese mismo string a DD/MM/AAAA para mostrar, sin re-parsearlo.
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
        getQuickFilterText: () => "",
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora",
        flex: 0.9,
        minWidth: 90,
        // `valueGetter` (no `field`+`valueFormatter`) a propósito: si lee
        // el string ISO crudo, AG Grid vuelve a inferirla como fecha (con
        // el mismo problema de la columna Fecha). Devolviendo ya el texto
        // "HH:MM", la columna se filtra/ordena como texto plano — mismo
        // comportamiento que el resto de columnas de texto, sin ícono raro.
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
        getQuickFilterText: () => "",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.3,
        minWidth: 130,
        getQuickFilterText: () => "",
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 90,
        filter: false,
        sortable: false,
        // Sin pinned a propósito: una columna fijada reserva el ancho del
        // scrollbar vertical aunque no haga falta scroll, dejando un
        // espacio en blanco justo antes de ella — con las 10 columnas ya
        // entrando sin scroll horizontal (ver el ajuste de anchos previo),
        // fijarla no aporta nada y sí ese espacio no deseado.
        cellRenderer: (p: ICellRendererParams<FilaActiva>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salidaIndividual(fila)}
            >
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [salidaIndividual],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaActiva>
            cargando={cargando}
            filtrosPorColumna
            id="activos"
            idFila={claveFilaActiva}
            claseFila={claseFilaActiva}
            columnas={columnas}
            filas={filasVisibles}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            accionesDerecha={<ToggleGafete filtro={filtroGafete} onCambiar={setFiltroGafete} />}
            controles={
              <>
                <button className="boton" title="Ctrl+N" onClick={onAbrirNuevoIngreso}>
                  + Ingreso
                </button>
                <button className="boton" title="Ctrl+S" onClick={onAbrirSalida}>
                  Salida
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, empresa…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {confirmarSalidaMasiva && (
        <Modal titulo="Confirmar salida" onCerrar={() => setConfirmarSalidaMasiva(false)}>
          <p style={{ marginTop: 0 }}>
            ¿Registrar la salida de {seleccionadas.length} persona(s)?
          </p>
          <ul style={{ margin: "0 0 1rem", paddingLeft: "1.2rem", color: "var(--muted)" }}>
            {seleccionadas.map((fila) => (
              <li key={claveFilaActiva(fila)}>
                {fila.contratista_nombre}
              </li>
            ))}
          </ul>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => setConfirmarSalidaMasiva(false)}
              disabled={procesando}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-primario"
              onClick={confirmarSalidasSeleccionadas}
              disabled={procesando}
            >
              {procesando ? "Registrando…" : "Confirmar"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
