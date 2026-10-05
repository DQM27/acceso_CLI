import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { History, Plus, UserCheck } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { listarHistorialVisitasSitio, listarVisitasActivas, registrarSalidaVisita } from "../api";
import type { MovimientoHistorialVisitaRemoto, MovimientoVisitaActivoResumen } from "../api";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const VisitaCheckInModal = lazy(() => import("./VisitaCheckInModal"));

type Vista = "activas" | "historial";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  activas: "Activas",
  historial: "Historial",
};

/** Mismos controles que Proveedores y Por correo (pedido del dueño
 * 2026-10-05): íconos del menú lateral con el relleno deslizante. */
const OPCIONES_VISTA: OpcionSegmentada<Vista>[] = [
  { valor: "activas", Icono: UserCheck, titulo: ETIQUETAS_VISTA.activas },
  { valor: "historial", Icono: History, titulo: ETIQUETAS_VISTA.historial },
];

function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <SegmentadoOpciones opciones={OPCIONES_VISTA} valor={vista} onCambiar={onCambiar} etiqueta="Vista" />
  );
}

/** Identidad de fila para el destello de celdas cambiadas (`idFila`). */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;
const idPorId = (fila: { id: number }) => String(fila.id);

/** Las dos grillas quedan montadas y sólo se ve la elegida (mismo motivo
 * que en Proveedores: no parpadea ni pierde scroll y filtros). */
function claseCapaVista(visible: boolean): string {
  return visible ? "capa-vista" : "capa-vista capa-vista-oculta";
}

/**
 * Lista de visitas activas + botón "+" (Nueva visita) que abre el check-in en un
 * modal -- mismo patrón que Activos/NuevoIngresoModal, sin ningún buscador
 * suelto sobre la pantalla (el único campo de búsqueda vive dentro del
 * modal; acá arriba de la grilla sólo va el filtro rápido, igual que
 * Activos). El toggle "Activas/Historial" cambia sólo qué consulta
 * trae la grilla -- misma pantalla, no una pantalla nueva:
 * - "Activas": `listar_visitas_activas`, quienes están adentro ahora mismo.
 * - "Historial": `historial_visitas_sitio` (sólo se llena en PC, ver
 *   `mobile/rust-core/src/lib.rs`) -- movimientos abiertos o cerrados de
 *   cualquier dispositivo del sitio.
 *
 * `refrescarSenal` (de `Shell`, mismo contador que ya usa Activos) hace que
 * la vista activa se recargue sola cuando llega cualquier sincronización
 * -- Realtime, pulso periódico o manual -- sin depender de que alguien
 * cambie de pestaña y vuelva.
 */
export default function Visitas({ refrescarSenal }: { refrescarSenal?: number }) {
  const [vista, setVista] = useState<Vista>("activas");
  const [filasActivas, setFilasActivas] = useState<MovimientoVisitaActivoResumen[]>([]);
  const [filasHistorial, setFilasHistorial] = useState<MovimientoHistorialVisitaRemoto[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");
  // Período del historial: mismo arranque que los demás historiales
  // ("Últimos 6 meses", `hasta` abierto).
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  // Para que la exportación lea lo que muestra la grilla (filtros incluidos).
  const tablaHistorialRef = useRef<TablaHandle<MovimientoHistorialVisitaRemoto>>(null);

  const total = vista === "activas" ? filasActivas.length : filasHistorial.length;
  useBarraEstado(cargando ? "Cargando…" : `${total} ${vista === "historial" ? "movimientos" : "visitantes adentro"}`);

  const recargarActivas = useCallback(() => {
    setCargando(true);
    return listarVisitasActivas()
      .then(setFilasActivas)
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return listarHistorialVisitasSitio(desde || undefined, hasta || undefined)
      .then(setFilasHistorial)
      .finally(() => setCargando(false));
  }, [desde, hasta]);

  useEffect(() => {
    let vigente = true;
    const recargar = vista === "activas" ? recargarActivas : recargarHistorial;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización (Realtime, pulso
    // periódico o manual) -- ver `App.tsx`/`Shell`. Sin esto, una visita
    // registrada desde otro dispositivo del sitio sólo aparecía acá
    // después de cambiar de pestaña y volver, aunque el resto de la app
    // (Activos) ya reaccionaba sola.
  }, [vista, refrescarSenal, recargarActivas, recargarHistorial]);

  const salida = useCallback(
    async (fila: MovimientoVisitaActivoResumen) => {
      try {
        await registrarSalidaVisita(fila.id);
        await recargarActivas();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargarActivas],
  );

  const columnasActivas: ColDef<MovimientoVisitaActivoResumen>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130 },
      { field: "motivo", headerName: "Motivo", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        colId: "fecha_entrada",
        type: "fecha",
        headerName: "Fecha",
        flex: 1,
        minWidth: 105,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_entrada) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrada",
        headerName: "Hora",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrada) : ""),
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 90,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<MovimientoVisitaActivoResumen>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salida(fila)}
            >
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [salida],
  );

  const columnasHistorial: ColDef<MovimientoHistorialVisitaRemoto>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      { field: "motivo", headerName: "Motivo", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        colId: "fecha_entrada",
        type: "fecha",
        headerName: "Fecha",
        flex: 1,
        minWidth: 105,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_entrada) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrada",
        headerName: "Entrada",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrada) : ""),
      },
      {
        colId: "hora_salida",
        headerName: "Salida",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : ""),
        valueFormatter: (p) => p.value || "—",
      },
      { field: "usuario_entrada_nombre", headerName: "Dio ingreso", flex: 1.1, minWidth: 120, valueFormatter: (p) => p.value ?? "—" },
      { field: "usuario_salida_nombre", headerName: "Dio salida", flex: 1.1, minWidth: 120, valueFormatter: (p) => p.value ?? "—" },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
          <div className={claseCapaVista(vista === "activas")}>
            <Tabla<MovimientoVisitaActivoResumen>
              cargando={cargando}
              filtrosPorColumna
              id="visitas-activas"
              idFila={idPorId}
              columnas={columnasActivas}
              filas={filasActivas}
              busqueda={busqueda}
              controles={
                <>
                  <button
                    type="button"
                    className="boton boton-icono"
                    title="Nueva visita"
                    aria-label="Nueva visita"
                    onClick={() => setModalAbierto(true)}
                  >
                    <Plus size={16} aria-hidden="true" />
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
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          </div>
          <div className={claseCapaVista(vista === "historial")}>
            <Tabla<MovimientoHistorialVisitaRemoto>
              ref={tablaHistorialRef}
              cargando={cargando}
              filtrosPorColumna
              id="visitas-historial"
              idFila={idPorUuid}
              columnas={columnasHistorial}
              filas={filasHistorial}
              busqueda={busqueda}
              controles={
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, empresa…"
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
                    nombreArchivo="historial-visitas"
                    titulo="Historial de Visitas"
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
          <VisitaCheckInModal
            onRegistrado={() => recargarActivas()}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
