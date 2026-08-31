import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import Modal from "../componentes/Modal";
import { historialGafete } from "../api";
import type { GafeteResumen, IncidenteGafete } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/**
 * Historial de incidentes de un gafete puntual — tabla simple (sin AG Grid,
 * la lista es chica: un puñado de filas por gafete) en su propio modal,
 * separado de `GestionGafeteModal` (que sólo maneja acciones) para no
 * mezclar "qué puedo hacer con este gafete" con "qué le pasó antes".
 */
export default function HistorialGafeteModal({
  gafete,
  onCerrar,
}: {
  gafete: GafeteResumen;
  onCerrar: () => void;
}) {
  const [historial, setHistorial] = useState<IncidenteGafete[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    historialGafete(gafete.id)
      .then(setHistorial)
      .catch((error) => setError(String(error)))
      .finally(() => setCargando(false));
  }, [gafete.id]);

  return (
    <Modal titulo={`Historial · Gafete ${String(gafete.numero).padStart(2, "0")}`} onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {!error && cargando && <p style={{ margin: 0, color: "var(--muted)" }}>Cargando…</p>}

        {!error && !cargando && historial.length === 0 && (
          <p style={{ margin: 0, color: "var(--muted)" }}>Este gafete no tiene incidentes registrados.</p>
        )}

        {!error && historial.length > 0 && (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
            <thead>
              <tr>
                <Encabezado>Fecha</Encabezado>
                <Encabezado>Hora</Encabezado>
                <Encabezado>Evento</Encabezado>
                <Encabezado>Operador</Encabezado>
                <Encabezado>Asignado a</Encabezado>
                <Encabezado>Motivo</Encabezado>
              </tr>
            </thead>
            <tbody>
              {historial.map((incidente) => (
                <tr key={incidente.id}>
                  <Celda>{textoFechaDDMMYYYY(fechaLocalYMD(incidente.fecha_hora))}</Celda>
                  <Celda>{textoHora(incidente.fecha_hora)}</Celda>
                  <Celda>{incidente.tipo === "Perdido" ? "Marcado perdido" : "Resuelto"}</Celda>
                  <Celda>{incidente.usuario_nombre}</Celda>
                  <Celda>{incidente.contratista_nombre ?? "—"}</Celda>
                  <Celda>{textoMotivo(incidente.motivo_resolucion)}</Celda>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cerrar
          </button>
        </div>
      </div>
    </Modal>
  );
}

function textoMotivo(motivo: IncidenteGafete["motivo_resolucion"]): string {
  switch (motivo) {
    case "Pagado":
      return "Pagado";
    case "Aparecido":
      return "Apareció";
    case null:
      return "—";
  }
}

function Encabezado({ children }: { children: ReactNode }) {
  return (
    <th
      style={{
        textAlign: "left",
        padding: "0.4rem 0.6rem",
        borderBottom: "1px solid var(--borde)",
        color: "var(--muted)",
        fontWeight: 500,
      }}
    >
      {children}
    </th>
  );
}

function Celda({ children }: { children: ReactNode }) {
  return (
    <td style={{ padding: "0.4rem 0.6rem", borderBottom: "1px solid var(--borde)" }}>
      {children}
    </td>
  );
}
