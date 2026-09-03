import { useCallback, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioContratista from "./FormularioContratista";
import { actualizarContratista, buscarContratistas, listarEmpresas } from "../api";
import type { ContratistaResumen, Empresa } from "../api";
import { textoFechaDDMMYYYY } from "../tiempo";

// "es de ruta"/"tiene acceso" se pueden tocar directo desde la grilla (ambos
// booleanos, bajo riesgo) — el resto (cédula, nombre, empresa, tipo, PRAIND)
// pasa por FormularioContratista (doble click en una fila para editar, botón
// "+ Nuevo" para dar de alta). Sin formulario de filtros a medida: el
// buscador de arriba (`busqueda`, quickFilterText de AG Grid) más los
// filtros nativos por columna (`filtrosPorColumna`, mismo enfoque que
// Historial) alcanzan — la grilla carga el universo completo una sola vez
// (ver `buscarContratistas`) y el resto pasa del lado del cliente. Los dos
// booleanos no llevan filtro de columna (`filter: false`): AG Grid Community
// no tiene un filtro booleano nativo decente, y ya se ven/tocan directo con
// el switch.
const columnas: ColDef<ContratistaResumen>[] = [
  { field: "cedula", headerName: "Cédula", flex: 1.4, minWidth: 140, cellStyle: { textAlign: "left" } },
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  { field: "empresa_nombre", headerName: "Empresa", flex: 1.4, minWidth: 140 },
  { field: "tipo_ingreso", headerName: "Tipo", flex: 1.2, minWidth: 120 },
  {
    field: "fecha_vencimiento_praind",
    headerName: "PRAIND vence",
    flex: 1.4,
    minWidth: 140,
    valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
  },
  {
    field: "es_personal_ruta",
    headerName: "Personal de ruta",
    // 140 truncaba el encabezado ("PERSONAL DE …") — 170 es lo que
    // necesita "PERSONAL DE RUTA" para no cortarse.
    flex: 1.7,
    minWidth: 170,
    cellRenderer: InterruptorCelda,
    filter: false,
  },
  {
    field: "tiene_acceso",
    headerName: "Acceso",
    flex: 1,
    minWidth: 100,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
    filter: false,
  },
];

export default function Contratistas() {
  const [empresas, setEmpresas] = useState<Empresa[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [filas, setFilas] = useState<ContratistaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [seleccionadas, setSeleccionadas] = useState<ContratistaResumen[]>([]);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | ContratistaResumen | null>(
    null,
  );

  useBarraEstado(
    cargando
      ? "Cargando…"
      : `${filas.length} resultado(s)` +
          (seleccionadas.length > 0 ? ` · ${seleccionadas.length} seleccionado(s)` : ""),
  );

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  useEffect(() => {
    listarEmpresas()
      .then(setEmpresas)
      .catch((error) => toast.error(String(error)));
  }, []);

  const recargar = useCallback((estaVigente: () => boolean = () => true) => {
    setCargando(true);
    return buscarContratistas()
      .then((pagina) => {
        if (!estaVigente()) return;
        setFilas(pagina.items);
      })
      .finally(() => {
        if (estaVigente()) setCargando(false);
      });
  }, []);

  useCargaAlCambiar(recargar);

  async function manejarEdicion(fila: ContratistaResumen) {
    try {
      await actualizarContratista(fila.id, {
        cedula: fila.cedula,
        nombre: fila.nombre,
        empresa_id: fila.empresa_id,
        tipo_ingreso: fila.tipo_ingreso,
        fecha_vencimiento_praind: fila.fecha_vencimiento_praind,
        es_personal_ruta: fila.es_personal_ruta,
        tiene_acceso: fila.tiene_acceso,
      });
    } catch (error) {
      // La grilla ya muestra el valor nuevo (edición optimista de AG Grid) —
      // si el guardado falla, hay que volver a pedir los datos reales para
      // que la celda no quede mintiendo.
      toast.error(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<ContratistaResumen>
            id="contratistas"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            controles={
              <>
                <button className="boton" title="Ctrl+N" onClick={() => setFormularioAbierto("crear")}>
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula o nombre…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioContratista
          contratista={formularioAbierto === "crear" ? undefined : formularioAbierto}
          empresas={empresas}
          onCerrar={() => setFormularioAbierto(null)}
          onGuardado={() => {
            setFormularioAbierto(null);
            recargar();
          }}
        />
      )}
    </div>
  );
}
