import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { listarRutasActivas, registrarRetornoRuta } from "../api";
import type { SalidaRutaActivaResumen } from "../api";
import { textoFechaDDMMYYYY, textoHora } from "../tiempo";

const SalidaRutaModal = lazy(() => import("./SalidaRutaModal"));

const ETIQUETAS_RESULTADO: Record<SalidaRutaActivaResumen["resultado"], string> = {
  Permitido: "Normal",
  PermitidoConAutorizacion: "Con autorización",
};

/**
 * Pantalla operativa de rutas -- mismo patrón que Visitas.tsx: grilla de
 * "activas" + botón "+ Salida" que abre el registro en un modal, y un botón
 * directo "Registrar retorno" por fila (sin diálogo de confirmación
 * intermedio -- mismo criterio que el botón "Salida" de Visitas: escritorio
 * es el camino de respaldo sin cámara, no tiene sentido agregar fricción
 * extra que el resto de la app no tiene en el mismo tipo de acción).
 * `refrescarSenal` (de `Shell`) recarga sola cuando llega cualquier
 * sincronización, igual que Activos/Visitas.
 */
export default function Rutas({ refrescarSenal }: { refrescarSenal?: number }) {
  const [filas, setFilas] = useState<SalidaRutaActivaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} ruta(s) activa(s)`);

  const recargar = useCallback(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- de lo contrario `react-hooks/set-state-in-effect` marca
    // esta actualización de estado como síncrona dentro del cuerpo del
    // efecto que la dispara (abajo). Ver el mismo comentario en Activos.tsx.
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarRutasActivas())
      .then(setFilas)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [refrescarSenal, recargar]);

  const registrarRetorno = useCallback(
    async (fila: SalidaRutaActivaResumen) => {
      try {
        await registrarRetornoRuta(fila.id);
        await recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  const columnas: ColDef<SalidaRutaActivaResumen>[] = useMemo(
    () => [
      { field: "vehiculo_placa", headerName: "Placa", flex: 1, minWidth: 100, cellStyle: { textAlign: "left" } },
      {
        field: "vehiculo_numero_unidad",
        headerName: "N.° unidad",
        flex: 0.9,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "—",
      },
      { field: "encargado_nombre", headerName: "Encargado", flex: 1.5, minWidth: 160, cellStyle: { textAlign: "left" } },
      { field: "numero_ruta", headerName: "Ruta", flex: 0.9, minWidth: 100 },
      { field: "sub_numero", headerName: "Sub", flex: 0.6, minWidth: 70 },
      { field: "numero_documento", headerName: "Documento", flex: 1.1, minWidth: 130 },
      {
        colId: "resultado",
        headerName: "Estado",
        flex: 1,
        minWidth: 120,
        valueGetter: (p) => (p.data ? ETIQUETAS_RESULTADO[p.data.resultado] : ""),
      },
      {
        colId: "fecha_salida",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? textoFechaDDMMYYYY(p.data.fecha_documento) : ""),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_salida) : ""),
      },
      { field: "usuario_salida_nombre", headerName: "Registró", flex: 1.1, minWidth: 120 },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<SalidaRutaActivaResumen>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => registrarRetorno(fila)}
            >
              Retorno
            </button>
          ) : null;
        },
      },
    ],
    [registrarRetorno],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<SalidaRutaActivaResumen>
            id="rutas-activas"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <>
                <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                  + Salida
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Placa, encargado, documento…"
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
          <SalidaRutaModal
            onRegistrado={() => {
              setModalAbierto(false);
              recargar();
            }}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
