import { useCallback, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioContratista from "./FormularioContratista";
import { actualizarContratista, buscarContratistas, listarEmpresas } from "../api";
import type {
  ContratistaResumen,
  Empresa,
  EstadoPraind,
  FiltroContratistas,
  TipoIngreso,
} from "../api";

// "es de ruta"/"tiene acceso" se pueden tocar directo desde la grilla (ambos
// booleanos, bajo riesgo) — el resto (cédula, nombre, empresa, tipo, PRAIND)
// pasa por FormularioContratista (doble click en una fila para editar, botón
// "+ Nuevo contratista" para dar de alta).
const columnas: ColDef<ContratistaResumen>[] = [
  { field: "cedula", headerName: "Cédula", width: 140, cellStyle: { textAlign: "left" } },
  { field: "nombre", headerName: "Nombre", flex: 1, cellStyle: { textAlign: "left" } },
  { field: "empresa_nombre", headerName: "Empresa", flex: 1 },
  { field: "tipo_ingreso", headerName: "Tipo", width: 120 },
  { field: "fecha_vencimiento_praind", headerName: "PRAIND vence", width: 140 },
  {
    field: "es_personal_ruta",
    headerName: "Personal de ruta",
    width: 140,
    cellDataType: "boolean",
    editable: true,
  },
  {
    field: "tiene_acceso",
    headerName: "Acceso",
    width: 100,
    cellDataType: "boolean",
    editable: true,
  },
];

const TIPOS: TipoIngreso[] = ["Praind", "InHouse", "PorCorreo", "Swat"];
const FILTRO_VACIO: FiltroContratistas = {};

export default function Contratistas() {
  const [filtro, setFiltro] = useState<FiltroContratistas>(FILTRO_VACIO);
  const [empresas, setEmpresas] = useState<Empresa[]>([]);
  const [filas, setFilas] = useState<ContratistaResumen[]>([]);
  const [total, setTotal] = useState(0);
  const [seleccionadas, setSeleccionadas] = useState<ContratistaResumen[]>([]);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | ContratistaResumen | null>(
    null,
  );

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  useEffect(() => {
    listarEmpresas()
      .then(setEmpresas)
      .catch((error) => console.error(error));
  }, []);

  const recargar = useCallback(() => {
    return buscarContratistas(filtro).then((pagina) => {
      setFilas(pagina.items);
      setTotal(pagina.total);
    });
  }, [filtro]);

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

  const tipoActual = filtro.tipos?.[0];

  const controlesDeFiltro = (
    <>
      <div className="campo" style={{ flex: "2 1 14rem" }}>
        Buscar
        <input
          placeholder="Cédula o nombre…"
          value={filtro.texto ?? ""}
          onChange={(evento) =>
            setFiltro((actual) => ({ ...actual, texto: evento.target.value || undefined }))
          }
        />
      </div>

      <div className="campo" style={{ flex: "1 1 9rem" }}>
        Empresa
        <select
          value={filtro.empresa_id ?? ""}
          onChange={(evento) =>
            setFiltro((actual) => ({
              ...actual,
              empresa_id: evento.target.value ? Number(evento.target.value) : undefined,
            }))
          }
        >
          <option value="">Todas</option>
          {empresas.map((empresa) => (
            <option key={empresa.id} value={empresa.id}>
              {empresa.nombre}
            </option>
          ))}
        </select>
      </div>

      <div className="campo" style={{ flex: "1 1 9rem" }}>
        Tipo
        <select
          value={tipoActual ?? ""}
          onChange={(evento) =>
            setFiltro((actual) => ({
              ...actual,
              tipos: evento.target.value ? [evento.target.value as TipoIngreso] : undefined,
            }))
          }
        >
          <option value="">Cualquiera</option>
          {TIPOS.map((tipo) => (
            <option key={tipo} value={tipo}>
              {tipo}
            </option>
          ))}
        </select>
      </div>

      <div className="campo" style={{ flex: "1 1 9rem" }}>
        PRAIND
        <select
          value={filtro.praind ?? ""}
          onChange={(evento) =>
            setFiltro((actual) => ({
              ...actual,
              praind: (evento.target.value || undefined) as EstadoPraind | undefined,
            }))
          }
        >
          <option value="">Cualquiera</option>
          <option value="vencido">Vencido</option>
          <option value="proximo">Próximo a vencer</option>
          <option value="sin_fecha">Sin fecha</option>
        </select>
      </div>

      {/* Personal de ruta es "sí o no" puro (nadie busca explícitamente
          "los que NO son de ruta") — checkbox. Acceso sí necesita el tercer
          estado (quiénes NO tienen acceso es una consulta real) — combo. */}
      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "0.4rem",
          alignSelf: "flex-end",
          paddingBottom: "0.6rem",
          color: "var(--texto)",
        }}
      >
        <input
          type="checkbox"
          checked={filtro.personal_ruta ?? false}
          onChange={(evento) =>
            setFiltro((actual) => ({
              ...actual,
              personal_ruta: evento.target.checked || undefined,
            }))
          }
        />
        Personal de ruta
      </label>

      <div className="campo" style={{ flex: "1 1 9rem" }}>
        Acceso
        <select
          value={filtro.tiene_acceso === undefined ? "" : filtro.tiene_acceso ? "si" : "no"}
          onChange={(evento) =>
            setFiltro((actual) => ({
              ...actual,
              tiene_acceso: evento.target.value === "" ? undefined : evento.target.value === "si",
            }))
          }
        >
          <option value="">Todos</option>
          <option value="si">Con acceso</option>
          <option value="no">Sin acceso</option>
        </select>
      </div>
    </>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado
        titulo="Contratistas"
        acciones={
          <button
            className="boton boton-primario"
            title="Ctrl+N"
            onClick={() => setFormularioAbierto("crear")}
          >
            + Nuevo contratista
          </button>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<ContratistaResumen>
            id="contratistas"
            columnas={columnas}
            filas={filas}
            controles={controlesDeFiltro}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {total} resultado(s)
          {seleccionadas.length > 0 && ` · ${seleccionadas.length} seleccionado(s)`}
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
