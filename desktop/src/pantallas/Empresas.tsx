import { useCallback, useEffect, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import FormularioEmpresa from "./FormularioEmpresa";
import { buscarEmpresas, establecerEmpresaActiva } from "../api";
import type { EmpresaResumen } from "../api";

const columnas: ColDef<EmpresaResumen>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1 },
  { field: "contratistas", headerName: "Contratistas", width: 130 },
  { field: "activo", headerName: "Activa", width: 100, cellDataType: "boolean", editable: true },
];

export default function Empresas() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<EmpresaResumen[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | EmpresaResumen | null>(
    null,
  );

  const recargar = useCallback(() => {
    return buscarEmpresas({ texto: texto || undefined }).then(setFilas);
  }, [texto]);

  useEffect(() => {
    let vigente = true;
    recargar()
      .then(() => vigente && setError(null))
      .catch((error) => vigente && setError(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);

  async function manejarEdicion(fila: EmpresaResumen) {
    try {
      await establecerEmpresaActiva(fila.id, fila.activo);
      setError(null);
    } catch (error) {
      setError(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "1rem" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: "0.75rem",
        }}
      >
        <h1 style={{ margin: 0, fontSize: "1.2rem", color: "var(--acento)" }}>Empresas</h1>
        <button className="boton boton-primario" onClick={() => setFormularioAbierto("crear")}>
          + Nueva empresa
        </button>
      </div>

      {error && <p style={{ color: "var(--error)" }}>{error}</p>}
      <div style={{ flex: 1, minHeight: 0 }}>
        <Tabla<EmpresaResumen>
          columnas={columnas}
          filas={filas}
          onCeldaEditada={manejarEdicion}
          onFilaDobleClic={setFormularioAbierto}
          controles={
            <div className="campo" style={{ flex: "1 1 16rem" }}>
              Buscar
              <input
                placeholder="Nombre…"
                value={texto}
                onChange={(evento) => setTexto(evento.target.value)}
              />
            </div>
          }
        />
      </div>

      {formularioAbierto && (
        <FormularioEmpresa
          empresa={formularioAbierto === "crear" ? undefined : formularioAbierto}
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
