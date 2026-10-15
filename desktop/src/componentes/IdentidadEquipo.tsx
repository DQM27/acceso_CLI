import { textoIdentidadEquipo } from "./IdentidadEquipo.logica";
import { useIdentidadEquipo } from "./useIdentidadEquipo";

/**
 * Unidad y etiqueta con que el panel registró esta PC (pantalla de login). Si
 * alguien la registró en la unidad equivocada, se ve acá antes de que la
 * sesión única empiece a cerrar sesiones.
 */
export default function IdentidadEquipo({
  recargar,
  className,
}: {
  recargar?: unknown;
  className?: string;
}) {
  const texto = textoIdentidadEquipo(useIdentidadEquipo(recargar));
  if (!texto) return null;
  return (
    <span className={className} title="Unidad y nombre con que este equipo quedó registrado">
      {texto}
    </span>
  );
}
