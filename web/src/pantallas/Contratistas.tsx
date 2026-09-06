import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Check } from "lucide-react";
import type { CellStyle, ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { actualizarAccesoContratista, listarContratistas } from "../api/contratistas";
import type { Contratista } from "../api/contratistas";
import { textoFechaDDMMYYYY } from "../tiempo";

// Declarados afuera del array de columnas y ya tipados como `CellStyle` --
// dentro del array, mezclar objetos literales con distintas claves
// (`textAlign` acá, `display`/`justifyContent`/`alignItems` en
// "es_personal_ruta") hace que TS infiera un único tipo combinado para
// todos los elementos y rechace la asignación a `ColDef<Contratista>[]`.
const ESTILO_IZQUIERDA: CellStyle = { textAlign: "left" };
const ESTILO_CENTRO_FLEX: CellStyle = { display: "flex", justifyContent: "center", alignItems: "center" };

/**
 * Vista + baja de contratistas (alcance pedido en
 * docs/plan-panel-administrativo-web.md, punto 3) -- sin alta ni edición
 * de los demás campos todavía, a propósito: eso es un formulario aparte
 * que no se pidió todavía (ver Contratistas.tsx de desktop/ si hace falta
 * calcarlo). El toggle "Activo" ES la baja (y la reactivación) -- global,
 * no por sitio, ver `api/contratistas.ts`.
 */
export default function Contratistas() {
  const [busqueda, setBusqueda] = useState("");
  const [filas, setFilas] = useState<Contratista[]>([]);
  const [cargando, setCargando] = useState(true);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarContratistas()
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
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000, "contratistas,empresas");

  async function manejarEdicion(fila: Contratista) {
    try {
      await actualizarAccesoContratista(fila.id, fila.activo);
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

  const columnas = useMemo<ColDef<Contratista>[]>(
    () => [
      {
        field: "identificacion",
        headerName: "Identificación",
        flex: 1.3,
        minWidth: 140,
        cellStyle: ESTILO_IZQUIERDA,
      },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: ESTILO_IZQUIERDA },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.3, minWidth: 140 },
      { field: "sitio_nombre", headerName: "Unidad operativa de origen", flex: 1.4, minWidth: 190 },
      { field: "tipo_ingreso", headerName: "Tipo", flex: 1.1, minWidth: 110 },
      {
        field: "fecha_vencimiento_praind",
        headerName: "PRAIND vence",
        flex: 1.3,
        minWidth: 130,
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        // Solo lectura a propósito: el alcance pedido acá es la vista y la
        // baja (columna "Activo" de abajo), no editar el resto de los
        // campos -- ver el doc-comment de arriba.
        field: "es_personal_ruta",
        headerName: "Personal de ruta",
        flex: 1.6,
        minWidth: 160,
        valueFormatter: (p) => (p.value ? "Sí" : "No"),
        // Render propio (no InterruptorCelda -- ese es el switch editable
        // que usa desktop/src/pantallas/Contratistas.tsx; acá es de sólo
        // lectura a propósito, ver el doc-comment de arriba) -- así el
        // ícono queda centrado y en verde cuando es "Sí" en vez del check
        // gris por defecto que AG Grid le pone a un campo booleano.
        cellStyle: ESTILO_CENTRO_FLEX,
        cellRenderer: ({ value }: { value: boolean }) =>
          value ? <Check size={16} color="var(--exito)" strokeWidth={2.5} aria-label="Sí" /> : null,
        filter: false,
      },
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
          <Tabla<Contratista>
            id="contratistas"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            onCeldaEditada={manejarEdicion}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder="Identificación o nombre…"
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
