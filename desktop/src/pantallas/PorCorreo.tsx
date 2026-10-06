import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { History, Plus, UserCheck } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarFilaCorreoActiva,
  claveFilaCorreoActiva,
  listarHistorialIngresosCorreoSitio,
  listarTodosLosCorreosActivos,
} from "../api/correo";
import type { FilaCorreoActiva, HistorialIngresoCorreoRemoto } from "../api/correo";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const IngresoCorreoModal = lazy(() => import("./IngresoCorreoModal"));

type Vista = "activos" | "historial";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  activos: "Activos",
  historial: "Historial",
};

// Duplicado a propósito de `Proveedores.tsx`/`Visitas.tsx` -- es puro
// markup de un botón, no vale la pena compartirlo entre pantallas que no
// se importan una a otra (ver la convención de App.tsx).
/** Opciones de vista -- íconos del menú lateral (UserCheck para Activos,
 * History para Historial). */
const OPCIONES_VISTA: OpcionSegmentada<Vista>[] = [
  { valor: "activos", Icono: UserCheck, titulo: ETIQUETAS_VISTA.activos },
  { valor: "historial", Icono: History, titulo: ETIQUETAS_VISTA.historial },
];

// Pieza única de íconos con el relleno deslizante (`SegmentadoOpciones`),
// pedido del usuario 2026-09-23.
function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <SegmentadoOpciones opciones={OPCIONES_VISTA} valor={vista} onCambiar={onCambiar} etiqueta="Vista" />
  );
}

/**
 * Pantalla "Por correo": ingresos de visitas autorizadas por correo
 * (generalmente entrevistas de RH), comodín mientras se termina el módulo de
 * Visitas. Copia adaptada de `Proveedores.tsx` (pedido del usuario
 * 2026-10-03): grilla de activos (locales + los que el otro dispositivo del
 * sitio tiene abiertos) con salida directa por fila, "+ Nuevo" que abre el
 * registro en un modal, e "Historial" de toda la unidad (caché
 * `historial_ingresos_correo_sitio`, exclusiva de escritorio).
 */
/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

/** Capa de cada vista (Activos / Historial): las dos grillas quedan
 * montadas una encima de la otra y sólo se muestra la elegida. Antes se
 * renderizaba una u otra, y cada cambio destruía la grilla y creaba otra
 * desde cero -- parpadeaba y perdía scroll y filtros (reportado por el
 * usuario 2026-09-23). La oculta conserva su tamaño (no `display: none`),
 * así AG Grid no recalcula columnas al volver; el cambio es un fundido
 * cruzado (`.capa-vista` en index.css). */
function claseCapaVista(visible: boolean): string {
  return visible ? "capa-vista" : "capa-vista capa-vista-oculta";
}

export default function PorCorreo({ refrescarSenal }: { refrescarSenal?: number }) {
  const [vista, setVista] = useState<Vista>("activos");
  // Período del historial -- mismo selector y mismo arranque ("Últimos 6
  // meses", `hasta` abierto) que Historial de contratistas (pedido del
  // usuario 2026-09-23: el selector en todos los historiales).
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  // Para que los botones de exportación lean lo que muestra la grilla.
  const tablaHistorialRef = useRef<TablaHandle<HistorialIngresoCorreoRemoto>>(null);
  const [filasActivos, setFilasActivos] = useState<FilaCorreoActiva[]>([]);
  const [filasHistorial, setFilasHistorial] = useState<HistorialIngresoCorreoRemoto[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  const total = vista === "activos" ? filasActivos.length : filasHistorial.length;
  useBarraEstado(
    cargando ? "Cargando…" : `${total} ${vista === "historial" ? "movimientos" : "ingreso(s) por correo activo(s)"}`,
  );

  const recargarActivos = useCallback(() => {
    setCargando(true);
    return listarTodosLosCorreosActivos()
      .then(setFilasActivos)
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return listarHistorialIngresosCorreoSitio(desde || undefined, hasta || undefined)
      .then(setFilasHistorial)
      .finally(() => setCargando(false));
  }, [desde, hasta]);

  useEffect(() => {
    let vigente = true;
    const recargar = vista === "activos" ? recargarActivos : recargarHistorial;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización: así aparece lo que
    // registró el otro dispositivo del sitio sin cambiar de pestaña.
  }, [vista, refrescarSenal, recargarActivos, recargarHistorial]);

  const registrarSalida = useCallback(
    async (fila: FilaCorreoActiva) => {
      try {
        await cerrarFilaCorreoActiva(fila);
        await recargarActivos();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargarActivos],
  );

  const columnasActivos: ColDef<FilaCorreoActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 0.9, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      { field: "motivo", headerName: "Motivo", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      {
        field: "placa",
        headerName: "Placa",
        flex: 0.8,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "Caminando",
      },
      { field: "gafete_numero", type: "numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
      {
        colId: "fecha_ingreso",
        type: "fecha",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<FilaCorreoActiva>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => registrarSalida(fila)}
            >
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [registrarSalida],
  );

  const columnasHistorial: ColDef<HistorialIngresoCorreoRemoto>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 0.9, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      {
        field: "motivo",
        headerName: "Motivo",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "placa",
        headerName: "Placa",
        flex: 0.8,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "Caminando",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.7,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_entrada",
        type: "fecha",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrada",
        headerName: "Entrada",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      {
        colId: "hora_salida",
        headerName: "Salida",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : ""),
        valueFormatter: (p) => p.value || "—",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "usuario_salida_nombre",
        headerName: "Dio salida",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => p.value ?? "—",
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
          <div className={claseCapaVista(vista === "activos")}>
            <Tabla<FilaCorreoActiva>
              cargando={cargando}
              filtrosPorColumna
              id="correo-activos"
              idFila={claveFilaCorreoActiva}
              columnas={columnasActivos}
              filas={filasActivos}
              busqueda={busqueda}
              controles={
                <>
                  <button
                    type="button"
                    className="boton boton-icono"
                    title="Nuevo ingreso por correo"
                    aria-label="Nuevo ingreso por correo"
                    onClick={() => setModalAbierto(true)}
                  >
                    <Plus size={16} aria-hidden="true" />
                  </button>
                  <div className="campo" style={{ flex: "0 1 16rem" }}>
                    <input
                      placeholder="Cédula, nombre, motivo…"
                      value={busqueda}
                      onChange={(evento) => setBusqueda(evento.target.value)}
                    />
                  </div>
                </>
              }
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          </div>
          <div className={claseCapaVista(vista === "historial")}>
            <Tabla<HistorialIngresoCorreoRemoto>
              ref={tablaHistorialRef}
              cargando={cargando}
              filtrosPorColumna
              id="correo-historial"
              idFila={idPorUuid}
              columnas={columnasHistorial}
              filas={filasHistorial}
              busqueda={busqueda}
              controles={
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, motivo…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              }
              accionesDerecha={
                <>
                  <SelectorRangoFecha
                    desde={desde}
                    hasta={hasta}
                    onAplicar={(nuevoDesde, nuevoHasta) => {
                      setDesde(nuevoDesde);
                      setHasta(nuevoHasta);
                    }}
                  />
                  <BotonesExportacion
                    tablaRef={tablaHistorialRef}
                    nombreArchivo="historial-por-correo"
                    titulo="Historial de ingresos por correo"
                    filtroDescripcion={`Filtro: ${textoRangoFecha(desde, hasta)}`}
                  />
                  <ToggleVista vista={vista} onCambiar={setVista} />
                </>
              }
            />
          </div>
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <IngresoCorreoModal
            onRegistrado={() => {
              setModalAbierto(false);
              recargarActivos();
            }}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
