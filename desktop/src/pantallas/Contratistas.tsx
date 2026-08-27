import { useEffect, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { buscarContratistas } from "../api";
import type { ContratistaResumen, UsuarioSesion } from "../api";

const columnas: ColDef<ContratistaResumen>[] = [
  { field: "cedula", headerName: "Cédula", width: 140 },
  { field: "nombre", headerName: "Nombre", flex: 1 },
  { field: "empresa_nombre", headerName: "Empresa", flex: 1 },
  { field: "tipo_ingreso", headerName: "Tipo", width: 120 },
  { field: "fecha_vencimiento_praind", headerName: "PRAIND vence", width: 140 },
  { field: "tiene_acceso", headerName: "Acceso", width: 100 },
  { field: "tiene_ingreso_activo", headerName: "Ingreso activo", width: 130 },
];

export default function Contratistas({
  sesion,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
}) {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<ContratistaResumen[]>([]);
  const [total, setTotal] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let vigente = true;
    buscarContratistas(texto)
      .then((pagina) => {
        if (!vigente) return;
        setFilas(pagina.items);
        setTotal(pagina.total);
        setError(null);
      })
      .catch((error) => vigente && setError(String(error)));
    return () => {
      vigente = false;
    };
  }, [texto]);

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
        <div>
          <strong style={{ color: "var(--acento)" }}>{sesion.nombre}</strong>{" "}
          <span style={{ color: "var(--muted)" }}>({sesion.rol})</span>
        </div>
        <button className="boton" onClick={onCerrarSesion}>
          Cerrar sesión
        </button>
      </div>
      <div className="campo" style={{ marginBottom: "0.75rem" }}>
        <input
          placeholder="Buscar por cédula o nombre…"
          value={texto}
          onChange={(evento) => setTexto(evento.target.value)}
        />
      </div>
      {error && <p style={{ color: "var(--error)" }}>{error}</p>}
      <div style={{ flex: 1 }}>
        <Tabla<ContratistaResumen> columnas={columnas} filas={filas} />
      </div>
      <p style={{ color: "var(--muted)", margin: "0.5rem 0 0" }}>{total} resultado(s)</p>
    </div>
  );
}
