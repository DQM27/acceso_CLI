import { TriangleAlert } from "lucide-react";

/**
 * Aviso pulsable de la barra de estado: "2 llevan más de 12 h adentro". Es
 * sólo presentación -- cuánta gente, con qué regla y qué texto lo decide el
 * núcleo (`domain::permanencia`); pulsarlo alterna el filtro a esas personas.
 * Ámbar para el aviso; rojo y con el ícono parpadeando para lo crítico (el
 * mismo latido que el punto de conexión caída).
 */
export default function AvisoPermanencia({
  texto,
  activo,
  critico = false,
  onAlternar,
}: {
  texto: string;
  /** El filtro a esas personas está puesto. */
  activo: boolean;
  /** Es el nivel crítico: rojo y con el ícono parpadeando. */
  critico?: boolean;
  onAlternar: () => void;
}) {
  return (
    <button
      type="button"
      className={critico ? "aviso-permanencia aviso-permanencia-critico" : "aviso-permanencia"}
      aria-pressed={activo}
      onClick={onAlternar}
      title={activo ? "Volver a ver a todos" : "Ver sólo a quienes llevan más tiempo adentro"}
    >
      <TriangleAlert
        size={14}
        strokeWidth={2}
        aria-hidden="true"
        className={critico ? "punto-conexion-parpadeo" : undefined}
      />
      {texto}
    </button>
  );
}
