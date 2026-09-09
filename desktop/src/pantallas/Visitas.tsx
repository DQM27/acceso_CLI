import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { listarVisitasActivas, registrarSalidaVisita } from "../api";
import type { MovimientoVisitaActivoResumen } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const VisitaCheckInModal = lazy(() => import("./VisitaCheckInModal"));

/**
 * Lista de visitas activas + botón "+ Visita" que abre el check-in en un
 * modal -- mismo patrón que Activos/NuevoIngresoModal, sin ningún buscador
 * suelto sobre la pantalla (el único campo de búsqueda vive dentro del
 * modal; acá arriba de la grilla sólo va el filtro por columna, igual que
 * Historial).
 */
export default function Visitas() {
  const [filas, setFilas] = useState<MovimientoVisitaActivoResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} visitantes adentro`);

  const recargar = useCallback(() => {
    setCargando(true);
    return listarVisitasActivas()
      .then(setFilas)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);

  const salida = useCallback(
    async (fila: MovimientoVisitaActivoResumen) => {
      try {
        await registrarSalidaVisita(fila.id);
        await recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  const columnas: ColDef<MovimientoVisitaActivoResumen>[] = useMemo(
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

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<MovimientoVisitaActivoResumen>
            id="visitas-activas"
            columnas={columnas}
            filas={filas}
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
          />
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <VisitaCheckInModal
            visitasActivas={filas}
            onRegistrado={() => recargar()}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
