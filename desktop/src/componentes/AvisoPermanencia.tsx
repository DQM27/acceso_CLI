import { TriangleAlert } from "lucide-react";

/**
 * Aviso pulsable de la barra de estado: "2 llevan más de 12 h adentro". Es
 * sólo presentación -- cuánta gente, con qué regla y qué texto lo decide
 * quien lo monta (Activos); pulsarlo alterna el filtro a esas personas.
 * Color de advertencia (ámbar): el rojo se reserva para lo crítico.
 */
export default function AvisoPermanencia({
  texto,
  activo,
  onAlternar,
}: {
  texto: string;
  /** El filtro a esas personas está puesto. */
  activo: boolean;
  onAlternar: () => void;
}) {
  return (
    <button
      type="button"
      className="aviso-permanencia"
      aria-pressed={activo}
      onClick={onAlternar}
      title={activo ? "Volver a ver a todos" : "Ver sólo a quienes llevan más tiempo adentro"}
    >
      <TriangleAlert size={14} strokeWidth={2} aria-hidden="true" />
      {texto}
    </button>
  );
}
