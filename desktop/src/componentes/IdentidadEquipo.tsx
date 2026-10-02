import { useEffect, useState } from "react";
import { obtenerIdentidadEquipo } from "../api/nube";
import type { IdentidadEquipo as Identidad } from "../api/nube";
import { textoIdentidadEquipo } from "./IdentidadEquipo.logica";

/**
 * Unidad y etiqueta con que el panel registró esta PC (login y barra de
 * estado). Si alguien la registró en la unidad equivocada, se ve acá antes
 * de que la sesión única empiece a cerrar sesiones. Se vuelve a leer cada
 * vez que cambia `recargar` (por ejemplo, al terminar una sincronización,
 * que es cuando puede llegar un token con datos nuevos). Un error al leer
 * no se muestra: es un dato informativo, nunca bloquea nada.
 */
export default function IdentidadEquipo({
  recargar,
  className,
}: {
  recargar?: unknown;
  className?: string;
}) {
  const [identidad, setIdentidad] = useState<Identidad | null>(null);
  useEffect(() => {
    let vigente = true;
    obtenerIdentidadEquipo()
      .then((leida) => {
        if (vigente) setIdentidad(leida);
      })
      .catch(() => {});
    return () => {
      vigente = false;
    };
  }, [recargar]);

  const texto = textoIdentidadEquipo(identidad);
  if (!texto) return null;
  return (
    <span className={className} title="Unidad y nombre con que este equipo quedó registrado">
      {texto}
    </span>
  );
}
