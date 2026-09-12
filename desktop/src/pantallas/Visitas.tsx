import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  listarAgendaVisitas,
  listarHistorialVisitasSitio,
  listarVisitasActivas,
  registrarSalidaVisita,
} from "../api";
import type {
  AgendaVisitaResumen,
  MovimientoHistorialVisitaRemoto,
  MovimientoVisitaActivoResumen,
} from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const VisitaCheckInModal = lazy(() => import("./VisitaCheckInModal"));
const AgendaCalendario = lazy(() => import("../componentes/AgendaCalendario"));

type Vista = "activas" | "historial" | "agenda";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  activas: "Activas",
  historial: "Historial",
  agenda: "Agenda",
};

function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <div style={{ display: "flex", gap: "0.25rem" }}>
      {(Object.keys(ETIQUETAS_VISTA) as Vista[]).map((opcion) => (
        <button
          key={opcion}
          type="button"
          className={opcion === vista ? "boton boton-primario" : "boton"}
          disabled={opcion === vista}
          onClick={() => onCambiar(opcion)}
        >
          {ETIQUETAS_VISTA[opcion]}
        </button>
      ))}
    </div>
  );
}

/**
 * Lista de visitas activas + botón "+ Visita" que abre el check-in en un
 * modal -- mismo patrón que Activos/NuevoIngresoModal, sin ningún buscador
 * suelto sobre la pantalla (el único campo de búsqueda vive dentro del
 * modal; acá arriba de la grilla sólo va el filtro rápido, igual que
 * Activos). El toggle "Activas/Historial/Agenda" cambia sólo qué consulta
 * trae la grilla -- misma pantalla, no una pantalla nueva:
 * - "Activas": `listar_visitas_activas`, quienes están adentro ahora mismo.
 * - "Historial": `historial_visitas_sitio` (sólo se llena en PC, ver
 *   `mobile/rust-core/src/lib.rs`) -- movimientos abiertos o cerrados de
 *   cualquier dispositivo del sitio.
 * - "Agenda": lectura pura de `citas`/`cita_visitantes` local, ya
 *   sincronizadas -- quién está programado desde hoy en adelante, sin
 *   tocar la nube de nuevo. De sólo lectura, no dispara ningún check-in.
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
  const [filasAgenda, setFilasAgenda] = useState<AgendaVisitaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  const total =
    vista === "activas"
      ? filasActivas.length
      : vista === "historial"
        ? filasHistorial.length
        : filasAgenda.length;
  useBarraEstado(cargando ? "Cargando…" : `${total} ${vista === "agenda" ? "programados" : vista === "historial" ? "movimientos" : "visitantes adentro"}`);

  const recargarActivas = useCallback(() => {
    setCargando(true);
    return listarVisitasActivas()
      .then(setFilasActivas)
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return listarHistorialVisitasSitio()
      .then(setFilasHistorial)
      .finally(() => setCargando(false));
  }, []);

  const recargarAgenda = useCallback(() => {
    setCargando(true);
    return listarAgendaVisitas()
      .then(setFilasAgenda)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    const recargar =
      vista === "activas" ? recargarActivas : vista === "historial" ? recargarHistorial : recargarAgenda;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización (Realtime, pulso
    // periódico o manual) -- ver `App.tsx`/`Shell`. Sin esto, una visita
    // registrada desde otro dispositivo del sitio sólo aparecía acá
    // después de cambiar de pestaña y volver, aunque el resto de la app
    // (Activos) ya reaccionaba sola.
  }, [vista, refrescarSenal, recargarActivas, recargarHistorial, recargarAgenda]);

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
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130 },
      { field: "motivo", headerName: "Motivo", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        colId: "fecha_entrada",
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
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "anfitrion_nombre", headerName: "Anfitrión", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      { field: "motivo", headerName: "Motivo", flex: 1.2, minWidth: 130, valueFormatter: (p) => p.value ?? "—" },
      {
        colId: "fecha_entrada",
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
      { field: "usuario_entrada_nombre", headerName: "Recibió", flex: 1.1, minWidth: 120, valueFormatter: (p) => p.value ?? "—" },
      { field: "usuario_salida_nombre", headerName: "Despidió", flex: 1.1, minWidth: 120, valueFormatter: (p) => p.value ?? "—" },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          {vista === "activas" && (
            <Tabla<MovimientoVisitaActivoResumen>
              id="visitas-activas"
              columnas={columnasActivas}
              filas={filasActivas}
              busqueda={busqueda}
              controles={
                <>
                  <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                    + Visita
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
          )}
          {vista === "historial" && (
            <Tabla<MovimientoHistorialVisitaRemoto>
              id="visitas-historial"
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
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          )}
          {vista === "agenda" && (
            <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: "0.6rem" }}>
              <div style={{ display: "flex", justifyContent: "flex-end" }}>
                <ToggleVista vista={vista} onCambiar={setVista} />
              </div>
              <div style={{ flex: 1, minHeight: 0 }}>
                <Suspense fallback={null}>
                  <AgendaCalendario filas={filasAgenda} />
                </Suspense>
              </div>
            </div>
          )}
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <VisitaCheckInModal
            visitasActivas={filasActivas}
            onRegistrado={() => recargarActivas()}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
