import { textoUnidadOperativa } from "./IdentidadEquipo.logica";
import { useIdentidadEquipo } from "./useIdentidadEquipo";

/** La unidad operativa de este equipo (`BRISAS`): bajo el nombre de la app en
 * el menú y en el login. */
export default function UnidadOperativa({
  recargar,
  className = "shell-unidad",
}: {
  recargar?: unknown;
  className?: string;
}) {
  const texto = textoUnidadOperativa(useIdentidadEquipo(recargar));
  if (!texto) return null;
  return (
    <span className={className} title="Unidad operativa de este equipo">
      {texto}
    </span>
  );
}
