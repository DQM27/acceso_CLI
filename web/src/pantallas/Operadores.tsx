import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { actualizarActivoUsuario, listarUsuarios } from "../api/usuarios";
import type { Usuario } from "../api/usuarios";

/**
 * Vista + baja de operadores/administradores globales -- calcada de
 * Contratistas.tsx (mismo alcance: sin alta ni edición de los demás campos,
 * eso vive en cada dispositivo). El toggle "Activo" ES la baja (y la
 * reactivación) -- global, no por sitio, ver `api/usuarios.ts`.
 */
export default function Operadores() {
  const [busqueda, setBusqueda] = useState("");
  const [filas, setFilas] = useState<Usuario[]>([]);
  const [cargando, setCargando] = useState(true);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarUsuarios()
      .then(setFilas)
      .catch((error) => {
        if (!silencioso) toast.error(String(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Cambia rara vez (altas/bajas puntuales) -- mismo intervalo que usan
  // desktop/mobile para su propio sync periódico.
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000);

  async function manejarEdicion(fila: Usuario) {
    try {
      await actualizarActivoUsuario(fila.id, fila.activo);
      toast.success(
        fila.activo ? `${fila.nombre} reactivado.` : `${fila.nombre} dado de baja.`,
      );
    } catch (error) {
      // La grilla ya muestra el valor nuevo (edición optimista de AG Grid) --
      // si el guardado falla, hay que volver a pedir los datos reales para
      // que la celda no quede mintiendo.
      toast.error(String(error));
      recargar();
    }
  }

  const columnas: ColDef<Usuario>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1, minWidth: 130, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "rol", headerName: "Rol", flex: 1, minWidth: 130 },
      { field: "sitio_nombre", headerName: "Sitio de origen", flex: 1.1, minWidth: 130 },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.9,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
        filter: false,
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<Usuario>
            id="operadores"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            onCeldaEditada={manejarEdicion}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder="Cédula o nombre…"
                  value={busqueda}
                  disabled={cargando}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
          />
        </div>
      </div>
    </div>
  );
}
