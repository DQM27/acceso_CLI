import { useCallback, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioContratista from "./FormularioContratista";
import { actualizarContratista, buscarContratistas, listarEmpresas } from "../api";
import type { ContratistaResumen, Empresa } from "../api";
import { textoFechaDDMMYYYY } from "../tiempo";

// "es de ruta"/"tiene acceso" se pueden tocar directo desde la grilla (ambos
// booleanos, bajo riesgo) — el resto (cédula, nombre, empresa, tipo, PRAIND)
// pasa por FormularioContratista (doble click en una fila para editar, botón
// "+ Nuevo" para dar de alta). Sin filtro propio de la pantalla: filtra/
// ordena con los filtros nativos de AG Grid por columna (mismo enfoque que
// Historial) en vez de un formulario de filtros a medida — la grilla carga
// el universo completo una sola vez (ver `buscarContratistas`) y el resto
// pasa del lado del cliente. Los dos booleanos no llevan filtro de columna
// (`filter: false`): AG Grid Community no tiene un filtro booleano nativo
// decente, y ya se ven/tocan directo con el switch.
const columnas: ColDef<ContratistaResumen>[] = [
  { field: "cedula", headerName: "Cédula", width: 140, cellStyle: { textAlign: "left" } },
  { field: "nombre", headerName: "Nombre", flex: 1, cellStyle: { textAlign: "left" } },
  { field: "empresa_nombre", headerName: "Empresa", flex: 1 },
  { field: "tipo_ingreso", headerName: "Tipo", width: 120 },
  {
    field: "fecha_vencimiento_praind",
    headerName: "PRAIND vence",
    width: 140,
    valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
  },
  {
    field: "es_personal_ruta",
    headerName: "Personal de ruta",
    width: 140,
    cellRenderer: InterruptorCelda,
    filter: false,
  },
  {
    field: "tiene_acceso",
    headerName: "Acceso",
    width: 100,
    cellRenderer: InterruptorCelda,
    filter: false,
  },
];

export default function Contratistas() {
  const [empresas, setEmpresas] = useState<Empresa[]>([]);
  const [filas, setFilas] = useState<ContratistaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [seleccionadas, setSeleccionadas] = useState<ContratistaResumen[]>([]);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | ContratistaResumen | null>(
    null,
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
      <PantallaEncabezado titulo="Contratistas" />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<ContratistaResumen>
            id="contratistas"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            controles={
              <button className="boton" title="Ctrl+N" onClick={() => setFormularioAbierto("crear")}>
                + Nuevo
              </button>
            }
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {cargando ? "Cargando…" : `${filas.length} resultado(s)`}
          {!cargando && seleccionadas.length > 0 && ` · ${seleccionadas.length} seleccionado(s)`}
        </p>
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
