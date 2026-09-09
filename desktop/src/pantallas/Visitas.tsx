import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  listarHistorialVisitasSitio,
  listarVisitasActivas,
  registrarSalidaVisita,
} from "../api";
import type { MovimientoHistorialVisitaRemoto, MovimientoVisitaActivoResumen } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const VisitaCheckInModal = lazy(() => import("./VisitaCheckInModal"));

type Vista = "activas" | "historial";

/**
 * Lista de visitas activas + botón "+ Visita" que abre el check-in en un
 * modal -- mismo patrón que Activos/NuevoIngresoModal, sin ningún buscador
 * suelto sobre la pantalla (el único campo de búsqueda vive dentro del
 * modal; acá arriba de la grilla sólo va el filtro por columna, igual que
 * Historial). El toggle "Activas/Historial" cambia sólo qué consulta trae
 * la grilla -- misma pantalla, no una pantalla nueva; "Historial" lee
 * `historial_visitas_sitio` (sólo se llena en PC, ver
 * `mobile/rust-core/src/lib.rs`), así que cierres/entradas de cualquier
 * dispositivo del sitio aparecen ahí, no sólo los de esta PC.
 */
export default function Visitas() {
  const [vista, setVista] = useState<Vista>("activas");
  const [filasActivas, setFilasActivas] = useState<MovimientoVisitaActivoResumen[]>([]);
  const [filasHistorial, setFilasHistorial] = useState<MovimientoHistorialVisitaRemoto[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  useBarraEstado(
    cargando
      ? "Cargando…"
      : vista === "activas"
        ? `${filasActivas.length} visitantes adentro`
        : `${filasHistorial.length} movimientos`,
  );

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

  useEffect(() => {
    let vigente = true;
    const recargar = vista === "activas" ? recargarActivas : recargarHistorial;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [vista, recargarActivas, recargarHistorial]);

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
        cellRenderer: (p: ICellRendererParams<MovimientoVisitaActivoResumen>) =>
          p.data ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salida(p.data!)}
            >
              Salida
            </button>
          ) : null,
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
          {vista === "activas" ? (
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
              accionesDerecha={
                <div style={{ display: "flex", gap: "0.25rem" }}>
                  <button type="button" className="boton boton-primario" disabled>
                    Activas
                  </button>
                  <button type="button" className="boton" onClick={() => setVista("historial")}>
                    Historial
                  </button>
                </div>
              }
            />
          ) : (
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
              accionesDerecha={
                <div style={{ display: "flex", gap: "0.25rem" }}>
                  <button type="button" className="boton" onClick={() => setVista("activas")}>
                    Activas
                  </button>
                  <button type="button" className="boton boton-primario" disabled>
                    Historial
                  </button>
                </div>
              }
            />
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
