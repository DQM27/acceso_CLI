import { textoUnidadOperativa } from "./IdentidadEquipo.logica";
import { useIdentidadEquipo } from "./useIdentidadEquipo";

/** La unidad operativa de este equipo (`BRISAS`), bajo el nombre de la app. */
export default function UnidadOperativa({ recargar }: { recargar?: unknown }) {
  const texto = textoUnidadOperativa(useIdentidadEquipo(recargar));
  if (!texto) return null;
  return (
    <span className="shell-unidad" title="Unidad operativa de este equipo">
      {texto}
    </span>
  );
}
