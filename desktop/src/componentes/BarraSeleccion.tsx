import { useState } from "react";
import type { ReactNode } from "react";
import { X } from "lucide-react";
import { usePresencia } from "./usePresencia";

/** Lo que dura la animación de salida (ver `.barra-seleccion-saliendo` en
 * `index.css`). */
const DURACION_SALIDA_MS = 140;

/**
 * Barra flotante con la acción para las filas seleccionadas ("3
 * seleccionados · Registrar salida · ✕"). Flota sobre la parte baja de la
 * tabla en vez de ocupar un lugar propio, así seleccionar filas no corre la
 * pantalla; entra deslizando desde abajo y sale con fundido. Es sólo
 * presentación: qué hace la acción lo decide quien la monta.
 *
 * El contenedor donde se monta tiene que ser `position: relative`.
 */
export default function BarraSeleccion({
  cantidad,
  etiqueta,
  accion,
  critica = false,
  onAccion,
  onLimpiar,
}: {
  cantidad: number;
  /** Cómo se llama lo seleccionado, en singular y plural (ej. "seleccionado" /
   * "seleccionados"); se elige según la cantidad. */
  etiqueta: { singular: string; plural: string };
  /** Contenido del botón de acción (ícono y texto). */
  accion: ReactNode;
  /** La acción es determinante y no se deshace a la ligera (ej. registrar
   * salidas): el botón va en rojo en vez del acento. */
  critica?: boolean;
  onAccion: () => void;
  onLimpiar: () => void;
}) {
  const { montado, saliendo } = usePresencia(cantidad > 0, DURACION_SALIDA_MS);

  // Mientras sale, la cantidad real ya es 0: se muestra la última para que el
  // texto no cambie a "0 seleccionados" durante el fundido.
  const [mostrada, setMostrada] = useState(cantidad);
  if (cantidad > 0 && cantidad !== mostrada) setMostrada(cantidad);

  if (!montado) return null;

  return (
    <div
      className={saliendo ? "barra-seleccion barra-seleccion-saliendo" : "barra-seleccion"}
      role="region"
      aria-label="Acciones para la selección"
    >
      <span className="barra-seleccion-texto" aria-live="polite">
        {mostrada} {mostrada === 1 ? etiqueta.singular : etiqueta.plural}
      </span>
      <button
        type="button"
        className={`boton ${critica ? "boton-peligro-solido" : "boton-primario"} boton-icono`}
        onClick={onAccion}
      >
        {accion}
      </button>
      <button
        type="button"
        className="boton boton-icono"
        onClick={onLimpiar}
        title="Quitar la selección"
        aria-label="Quitar la selección"
      >
        <X size={16} aria-hidden="true" />
      </button>
    </div>
  );
}
