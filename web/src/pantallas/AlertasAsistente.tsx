import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ShieldAlert } from "lucide-react";
import { listarAlertas, marcarAlertaRevisada, type AlertaIa } from "../api/lienzo";
import { mensajeError } from "../mensajeError";

const ORIGEN: Record<AlertaIa["origen"], string> = {
  detector: "Lo escribió en el chat",
  modelo: "Lo marcó el asistente",
  parser: "Consulta fuera de lo permitido",
};

const fecha = new Intl.DateTimeFormat("es-CR", { dateStyle: "medium", timeStyle: "short", timeZone: "America/Costa_Rica" });

/**
 * Intentos de que el asistente ejecutara SQL o se saltara sus reglas: quién fue,
 * qué escribió y por qué se marcó. Sólo las ve la cuenta con el alcance
 * `alertas` (la base no devuelve nada a las demás).
 */
export default function AlertasAsistente() {
  const cliente = useQueryClient();
  const { data, isLoading, error } = useQuery({ queryKey: ["lienzo-alertas"], queryFn: listarAlertas });
  const revisar = useMutation({
    mutationFn: (id: string) => marcarAlertaRevisada(id),
    onSuccess: () => cliente.invalidateQueries({ queryKey: ["lienzo-alertas"] }),
  });

  if (error) return <p className="asistente-error" role="alert">{mensajeError(error)}</p>;
  if (isLoading || !data) return <p role="status">Cargando…</p>;
  if (data.length === 0) return <p className="asistente-vacio">Sin alertas: nadie ha intentado nada raro.</p>;

  return (
    <ul className="asistente-alertas" aria-label="Alertas del asistente">
      {data.map((a) => (
        <li key={a.id} className={`asistente-alerta${a.revisada_en ? " asistente-alerta-revisada" : ""}`}>
          <header>
            <ShieldAlert size={16} aria-hidden="true" />
            <strong>{a.correo}</strong>
            <span className="asistente-alerta-fecha">{fecha.format(new Date(a.creado_en))}</span>
            <span className="asistente-espacio" />
            {a.revisada_en ? (
              <span className="asistente-alerta-ok">
                <Check size={14} aria-hidden="true" /> Revisada
              </span>
            ) : (
              <button type="button" className="boton" onClick={() => revisar.mutate(a.id)} disabled={revisar.isPending}>
                Marcar revisada
              </button>
            )}
          </header>
          <p className="asistente-alerta-motivo">
            {ORIGEN[a.origen]}: {a.motivo}
          </p>
          <blockquote>{a.mensaje}</blockquote>
          {a.detalle && <pre>{a.detalle}</pre>}
        </li>
      ))}
    </ul>
  );
}
