import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { CalendarClock, History, LogIn, Plus, UserCheck } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarVisitaRemota,
  listarAgendaVisitas,
  listarHistorialVisitasSitio,
  listarVisitasActivas,
  listarVisitasRemotas,
  registrarSalidaVisita,
} from "../api";
import {
  cerrarFilaCorreoActiva,
  listarHistorialIngresosCorreoSitio,
  listarTodosLosCorreosActivos,
} from "../api/correo";
import { fechaHaceMeses, fechaLocalYMD, fechaYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  TEXTO_ORIGEN,
  textoMedioDePlaca,
  unirAdentro,
  unirHistorial,
  visitasEsperadasHoy,
} from "./Visitas.logica";
import type { FilaEsperada, FilaVisitaAdentro, FilaVisitaHistorial } from "./Visitas.logica";

const NuevaVisitaModal = lazy(() => import("./NuevaVisitaModal"));

type Vista = "esperadas" | "adentro" | "historial";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  esperadas: "Esperadas hoy",
  adentro: "Adentro",
  historial: "Historial",
};

/** Mismos controles que el resto de las vistas: íconos con el relleno
 * deslizante (`SegmentadoOpciones`). */
const OPCIONES_VISTA: OpcionSegmentada<Vista>[] = [
  { valor: "esperadas", Icono: CalendarClock, titulo: ETIQUETAS_VISTA.esperadas },
  { valor: "adentro", Icono: UserCheck, titulo: ETIQUETAS_VISTA.adentro },
  { valor: "historial", Icono: History, titulo: ETIQUETAS_VISTA.historial },
];

function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <SegmentadoOpciones opciones={OPCIONES_VISTA} valor={vista} onCambiar={onCambiar} etiqueta="Vista" />
  );
}

/** Identidad de fila para el destello de celdas cambiadas (`idFila`). */
const idPorClave = (fila: { clave: string }) => fila.clave;

/** Las tres grillas quedan montadas y sólo se ve la elegida (mismo motivo
 * que en Proveedores: no parpadea ni pierde scroll y filtros). */
function claseCapaVista(visible: boolean): string {
  return visible ? "capa-vista" : "capa-vista capa-vista-oculta";
}

const guion = (valor: string | null | undefined) => valor || "—";

const BOTON_FILA = { padding: "0.15rem 0.55rem", fontSize: "0.78rem" } as const;

/**
 * Visitas: las agendadas en la web (camino principal) y las autorizadas por
 * correo (las registra el guarda a mano), en una sola pantalla con tres
 * vistas (pedido del dueño 2026-10-05):
 * - "Esperadas hoy": citas de hoy que todavía no entraron, apenas se
 *   agendan (llegan por la sincronización), con "Dar entrada".
 * - "Adentro": quienes están adentro ahora, de los dos tipos, con salida.
 * - "Historial": movimientos de los dos tipos de toda la unidad, con
 *   período y exportación.
 * El botón "+" y "Dar entrada" abren el mismo modal (`NuevaVisitaModal`).
 *
 * `refrescarSenal` (de `Shell`) recarga la vista visible en cada
 * sincronización: lo que registra o agenda otro equipo aparece solo.
 */
export default function Visitas({ refrescarSenal }: { refrescarSenal?: number }) {
  const [vista, setVista] = useState<Vista>("esperadas");
  const [esperadas, setEsperadas] = useState<FilaEsperada[]>([]);
  const [adentro, setAdentro] = useState<FilaVisitaAdentro[]>([]);
  const [historial, setHistorial] = useState<FilaVisitaHistorial[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modal, setModal] = useState<{ cedula?: string } | null>(null);
  const [busqueda, setBusqueda] = useState("");
  // Período del historial: mismo arranque que los demás historiales
  // ("Últimos 6 meses", `hasta` abierto).
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  // Para que la exportación lea lo que muestra la grilla (filtros incluidos).
  const tablaHistorialRef = useRef<TablaHandle<FilaVisitaHistorial>>(null);

  const total = vista === "esperadas" ? esperadas.length : vista === "adentro" ? adentro.length : historial.length;
  const unidad =
    vista === "esperadas" ? "visita(s) esperada(s) hoy" : vista === "adentro" ? "visitante(s) adentro" : "movimientos";
  useBarraEstado(cargando ? "Cargando…" : `${total} ${unidad}`);

  const recargarEsperadas = useCallback(() => {
    setCargando(true);
    const hoy = fechaYMD(new Date());
    return Promise.all([
      listarAgendaVisitas(),
      listarVisitasActivas(),
      listarVisitasRemotas(),
      listarTodosLosCorreosActivos(),
      listarHistorialVisitasSitio(hoy, hoy),
    ])
      .then(([agenda, visitas, remotas, correos, historialHoy]) =>
        setEsperadas(visitasEsperadasHoy(agenda, [...visitas, ...remotas, ...correos], historialHoy, hoy)),
      )
      .finally(() => setCargando(false));
  }, []);

  const recargarAdentro = useCallback(() => {
    setCargando(true);
    return Promise.all([listarVisitasActivas(), listarVisitasRemotas(), listarTodosLosCorreosActivos()])
      .then(([visitas, remotas, correos]) => setAdentro(unirAdentro(visitas, remotas, correos)))
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return Promise.all([
      listarHistorialVisitasSitio(desde || undefined, hasta || undefined),
      listarHistorialIngresosCorreoSitio(desde || undefined, hasta || undefined),
    ])
      .then(([visitas, correos]) => setHistorial(unirHistorial(visitas, correos)))
      .finally(() => setCargando(false));
  }, [desde, hasta]);

  const recargarVista =
    vista === "esperadas" ? recargarEsperadas : vista === "adentro" ? recargarAdentro : recargarHistorial;

  useEffect(() => {
    let vigente = true;
    const recargar =
      vista === "esperadas" ? recargarEsperadas : vista === "adentro" ? recargarAdentro : recargarHistorial;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización (Realtime, pulso
    // periódico o manual) -- ver `App.tsx`/`Shell`.
  }, [vista, refrescarSenal, recargarEsperadas, recargarAdentro, recargarHistorial]);

  const darSalida = useCallback(
    async (fila: FilaVisitaAdentro) => {
      try {
        if (fila.fuente.tipo === "visita") await registrarSalidaVisita(fila.fuente.id);
        else if (fila.fuente.tipo === "visita_remota") await cerrarVisitaRemota(fila.fuente.uuid);
        else await cerrarFilaCorreoActiva(fila.fuente.fila);
        await recargarAdentro();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargarAdentro],
  );

  const columnasEsperadas: ColDef<FilaEsperada>[] = useMemo(
    () => [
      {
        field: "hora_estimada",
        headerName: "Hora",
        flex: 0.7,
        minWidth: 80,
        valueFormatter: (p) => (p.value ? String(p.value).slice(0, 5) : "—"),
      },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 120, valueFormatter: (p) => guion(p.value) },
      {
        colId: "medio",
        headerName: "Medio de ingreso",
        flex: 1,
        minWidth: 130,
        // Lo que escribió el anfitrión en la cita.
        valueGetter: (p) => (p.data ? textoMedioDePlaca(p.data.placa_vehiculo) : ""),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130 },
      { field: "motivo", headerName: "Motivo", flex: 1.3, minWidth: 130, valueFormatter: (p) => guion(p.value) },
      {
        colId: "vigencia",
        headerName: "Vigencia",
        flex: 1,
        minWidth: 120,
        valueGetter: (p) =>
          !p.data || p.data.fecha_desde === p.data.fecha_hasta
            ? "Sólo hoy"
            : `Hasta ${textoFechaDDMMYYYY(p.data.fecha_hasta)}`,
      },
      {
        colId: "llegada",
        headerName: "Llegada",
        flex: 1,
        minWidth: 110,
        valueGetter: (p) =>
          p.data?.llegada.tipo === "salio" ? `Salió ${textoHora(p.data.llegada.hora)}` : "Sin llegar",
      },
      {
        headerName: "Acción",
        flex: 1,
        minWidth: 120,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<FilaEsperada>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton boton-primario"
              style={BOTON_FILA}
              onClick={() => setModal({ cedula: fila.cedula })}
            >
              <LogIn size={14} aria-hidden="true" /> Dar entrada
            </button>
          ) : null;
        },
      },
    ],
    [],
  );

  const columnasAdentro: ColDef<FilaVisitaAdentro>[] = useMemo(
    () => [
      {
        field: "origen",
        headerName: "Origen",
        flex: 0.9,
        minWidth: 105,
        valueGetter: (p) => (p.data ? TEXTO_ORIGEN[p.data.origen] : ""),
      },
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 120, valueFormatter: (p) => guion(p.value) },
      { field: "anfitrion", headerName: "Anfitrión", flex: 1.1, minWidth: 120, valueFormatter: (p) => guion(p.value) },
      { field: "motivo", headerName: "Motivo", flex: 1.4, minWidth: 140, valueFormatter: (p) => guion(p.value) },
      {
        colId: "medio",
        headerName: "Medio de ingreso",
        flex: 1,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoMedioDePlaca(p.data.placa) : ""),
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.7,
        minWidth: 85,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
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
        flex: 0.7,
        minWidth: 80,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrada) : ""),
      },
      {
        headerName: "Acción",
        flex: 0.8,
        minWidth: 90,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<FilaVisitaAdentro>) => {
          const fila = p.data;
          return fila ? (
            <button type="button" className="boton" style={BOTON_FILA} onClick={() => darSalida(fila)}>
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [darSalida],
  );

  const columnasHistorial: ColDef<FilaVisitaHistorial>[] = useMemo(
    () => [
      {
        field: "origen",
        headerName: "Origen",
        flex: 0.9,
        minWidth: 105,
        valueGetter: (p) => (p.data ? TEXTO_ORIGEN[p.data.origen] : ""),
      },
      { field: "cedula", headerName: "Cédula", flex: 1.1, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "empresa", headerName: "Empresa", flex: 1.1, minWidth: 120, valueFormatter: (p) => guion(p.value) },
      { field: "anfitrion", headerName: "Anfitrión", flex: 1.1, minWidth: 120, valueFormatter: (p) => guion(p.value) },
      { field: "motivo", headerName: "Motivo", flex: 1.4, minWidth: 140, valueFormatter: (p) => guion(p.value) },
      {
        colId: "medio",
        headerName: "Medio de ingreso",
        flex: 1,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoMedioDePlaca(p.data.placa) : ""),
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.7,
        minWidth: 85,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
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
        flex: 0.7,
        minWidth: 80,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrada) : ""),
      },
      {
        colId: "hora_salida",
        headerName: "Salida",
        flex: 0.7,
        minWidth: 80,
        valueGetter: (p) => (p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : ""),
        valueFormatter: (p) => p.value || "—",
      },
      {
        field: "usuario_entrada_nombre",
        headerName: "Dio ingreso",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => guion(p.value),
      },
      {
        field: "usuario_salida_nombre",
        headerName: "Dio salida",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => guion(p.value),
      },
    ],
    [],
  );

  const buscador = (
    <div className="campo" style={{ flex: "0 1 16rem" }}>
      <input
        placeholder="Cédula, nombre, anfitrión…"
        value={busqueda}
        onChange={(evento) => setBusqueda(evento.target.value)}
      />
    </div>
  );
  const botonNueva = (
    <button
      type="button"
      className="boton boton-icono"
      title="Nueva visita"
      aria-label="Nueva visita"
      onClick={() => setModal({})}
    >
      <Plus size={16} aria-hidden="true" />
    </button>
  );
  const toggle = <ToggleVista vista={vista} onCambiar={setVista} />;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
          <div className={claseCapaVista(vista === "esperadas")}>
            <Tabla<FilaEsperada>
              cargando={cargando}
              filtrosPorColumna
              id="visitas-esperadas"
              idFila={idPorClave}
              columnas={columnasEsperadas}
              filas={esperadas}
              busqueda={busqueda}
              controles={
                <>
                  {botonNueva}
                  {buscador}
                </>
              }
              accionesDerecha={toggle}
            />
          </div>
          <div className={claseCapaVista(vista === "adentro")}>
            <Tabla<FilaVisitaAdentro>
              cargando={cargando}
              filtrosPorColumna
              id="visitas-adentro"
              idFila={idPorClave}
              columnas={columnasAdentro}
              filas={adentro}
              busqueda={busqueda}
              controles={
                <>
                  {botonNueva}
                  {buscador}
                </>
              }
              accionesDerecha={toggle}
            />
          </div>
          <div className={claseCapaVista(vista === "historial")}>
            <Tabla<FilaVisitaHistorial>
              ref={tablaHistorialRef}
              cargando={cargando}
              filtrosPorColumna
              id="visitas-historial-unificado"
              idFila={idPorClave}
              columnas={columnasHistorial}
              filas={historial}
              busqueda={busqueda}
              controles={buscador}
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
                  {toggle}
                </>
              }
            />
          </div>
        </div>
      </div>

      <Suspense fallback={null}>
        {modal && (
          <NuevaVisitaModal
            cedulaInicial={modal.cedula}
            onRegistrado={() => {
              recargarVista().catch((error) => toast.error(String(error)));
            }}
            onCerrar={() => setModal(null)}
          />
        )}
      </Suspense>
    </div>
  );
}
