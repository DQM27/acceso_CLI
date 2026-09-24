import { useCallback, useState } from "react";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioEmpresa from "./FormularioEmpresa";
import FormularioEmpresaProveedor from "./FormularioEmpresaProveedor";
import { buscarEmpresas, establecerEmpresaActiva } from "../api";
import type { EmpresaResumen } from "../api";
import { buscarEmpresasProveedor, establecerEmpresaProveedorActiva } from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";

type TipoEmpresa = "contratista" | "proveedor";
type FilaEmpresa = EmpresaResumen | EmpresaProveedor;

const columnasContratista: ColDef<EmpresaResumen>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  { field: "contratistas", headerName: "Contratistas", flex: 1.3, minWidth: 130 },
  {
    field: "activo",
    headerName: "Activa",
    flex: 1,
    minWidth: 100,
    filter: false,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
  },
];

const columnasProveedor: ColDef<EmpresaProveedor>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  {
    field: "activo",
    headerName: "Activa",
    flex: 1,
    minWidth: 100,
    filter: false,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
  },
];

/**
 * Una sola pestaña "Empresas" con selector de tipo (contratista/proveedor),
 * mismo criterio que el filtro "Tipo" de Gafetes.tsx -- son dos catálogos
 * separados en el núcleo (`empresas`/`empresas_proveedor`, ver
 * docs/features-futuras/plan-control-proveedores.md), pero no ameritan dos
 * pestañas de sidebar aparte, son la misma clase de pantalla (catálogo con
 * alta + activar/desactivar). Proveedor no admite renombrar -- el núcleo no
 * lo expone (ver FormularioEmpresaProveedor), sólo activar/desactivar
 * inline en la grilla.
 */
export default function Empresas() {
  const [tipo, setTipo] = useState<TipoEmpresa>("contratista");
  const [texto, setTexto] = useState("");
  const [filasContratista, setFilasContratista] = useState<EmpresaResumen[]>([]);
  const [filasProveedor, setFilasProveedor] = useState<EmpresaProveedor[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | EmpresaResumen | null>(
    null,
  );

  const filas: FilaEmpresa[] = tipo === "contratista" ? filasContratista : filasProveedor;
  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      const promesa =
        tipo === "contratista"
          ? buscarEmpresas({ texto: texto || undefined }).then((datos) => {
              if (estaVigente()) setFilasContratista(datos);
            })
          : buscarEmpresasProveedor(texto).then((datos) => {
              if (estaVigente()) setFilasProveedor(datos);
            });
      return promesa.finally(() => {
        if (estaVigente()) setCargando(false);
      });
    },
    [texto, tipo],
  );

  useCargaAlCambiar(recargar, true);

  async function manejarEdicion(fila: FilaEmpresa) {
    try {
      if (tipo === "contratista") {
        await establecerEmpresaActiva(fila.id, fila.activo);
      } else {
        await establecerEmpresaProveedorActiva(fila.id, fila.activo);
      }
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  // Orden pedido por el usuario (2026-09-23): "+ Nueva", el buscador y al
  // final el selector de tipo.
  const controles = (
    <>
      <button
        type="button"
        className="boton boton-icono"
        title="Nueva empresa"
        aria-label="Nueva empresa"
        onClick={() => setFormularioAbierto("crear")}
      >
        <Plus size={16} aria-hidden="true" />
      </button>
      <div className="campo" style={{ flex: "0 1 16rem" }}>
        <input
          placeholder="Nombre…"
          value={texto}
          onChange={(evento) => setTexto(evento.target.value)}
        />
      </div>
      <div className="campo" style={{ flex: "0 1 13rem" }}>
        <select
          value={tipo}
          onChange={(evento) => {
            setTipo(evento.target.value as TipoEmpresa);
            setTexto("");
          }}
        >
          <option value="contratista">Empresas (contratistas)</option>
          <option value="proveedor">Empresas proveedoras</option>
        </select>
      </div>
    </>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          {tipo === "contratista" ? (
            <Tabla<EmpresaResumen>
              cargando={cargando}
              filtrosPorColumna
              id="empresas"
              columnas={columnasContratista}
              filas={filasContratista}
              onCeldaEditada={manejarEdicion}
              onFilaDobleClic={setFormularioAbierto}
              controles={controles}
            />
          ) : (
            <Tabla<EmpresaProveedor>
              cargando={cargando}
              filtrosPorColumna
              id="empresas-proveedor"
              columnas={columnasProveedor}
              filas={filasProveedor}
              onCeldaEditada={manejarEdicion}
              controles={controles}
            />
          )}
        </div>
      </div>

      {formularioAbierto && tipo === "contratista" && (
        <FormularioEmpresa
          empresa={formularioAbierto === "crear" ? undefined : formularioAbierto}
          onCerrar={() => setFormularioAbierto(null)}
          onGuardado={() => {
            setFormularioAbierto(null);
            recargar();
          }}
        />
      )}

      {formularioAbierto && tipo === "proveedor" && (
        <FormularioEmpresaProveedor
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
