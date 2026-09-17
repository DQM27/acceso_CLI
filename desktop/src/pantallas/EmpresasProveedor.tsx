import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioEmpresaProveedor from "./FormularioEmpresaProveedor";
import { buscarEmpresasProveedor, establecerEmpresaProveedorActiva } from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";

const columnas: ColDef<EmpresaProveedor>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  {
    field: "activo",
    headerName: "Activa",
    flex: 1,
    minWidth: 100,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
  },
];

export default function EmpresasProveedor() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<EmpresaProveedor[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState(false);

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

  useHotkeys("ctrl+n", () => setFormularioAbierto(true), { preventDefault: true });

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      return buscarEmpresasProveedor(texto)
        .then((datos) => {
          if (estaVigente()) setFilas(datos);
        })
        .finally(() => {
          if (estaVigente()) setCargando(false);
        });
    },
    [texto],
  );

  useCargaAlCambiar(recargar, true);

  async function manejarEdicion(fila: EmpresaProveedor) {
    try {
      await establecerEmpresaProveedorActiva(fila.id, fila.activo);
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<EmpresaProveedor>
            id="empresas-proveedor"
            columnas={columnas}
            filas={filas}
            onCeldaEditada={manejarEdicion}
            controles={
              <>
                <button
                  className="boton"
                  title="Ctrl+N"
                  onClick={() => setFormularioAbierto(true)}
                >
                  + Nueva
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Nombre…"
                    value={texto}
                    onChange={(evento) => setTexto(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioEmpresaProveedor
          onCerrar={() => setFormularioAbierto(false)}
          onGuardado={() => {
            setFormularioAbierto(false);
            recargar();
          }}
        />
      )}
    </div>
  );
}
