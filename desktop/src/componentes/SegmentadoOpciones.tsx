import type { CSSProperties } from "react";
import type { LucideIcon } from "lucide-react";

export interface OpcionSegmentada<V extends string> {
  valor: V;
  Icono: LucideIcon;
  /** Nombre de la opción: se ve al pasar el mouse y lo lee el lector de
   * pantalla (el control sólo muestra íconos). */
  titulo: string;
}

/**
 * Control segmentado de íconos para elegir UNA opción (ej. "Activos /
 * Historial", filtro de gafete de Activos). El relleno de acento es un
 * indicador aparte que se desliza de una opción a otra en vez de saltar
 * (pedido del usuario 2026-09-23) -- ver `.segmentado-deslizante` en
 * index.css. Las opciones miden todas lo mismo, así el desplazamiento es
 * exactamente de una opción por paso.
 */
export default function SegmentadoOpciones<V extends string>({
  opciones,
  valor,
  onCambiar,
  etiqueta,
}: {
  opciones: OpcionSegmentada<V>[];
  valor: V;
  onCambiar: (valor: V) => void;
  /** Nombre del grupo para el lector de pantalla (ej. "Vista"). */
  etiqueta: string;
}) {
  const indice = Math.max(
    0,
    opciones.findIndex((opcion) => opcion.valor === valor),
  );
  const estilo = { "--cantidad": opciones.length, "--indice": indice } as CSSProperties;

  return (
    <div className="segmentado segmentado-deslizante" role="group" aria-label={etiqueta} style={estilo}>
      <span className="segmentado-indicador" aria-hidden="true" />
      {opciones.map(({ valor: opcion, Icono, titulo }) => (
        <button
          key={opcion}
          type="button"
          title={titulo}
          aria-label={titulo}
          aria-pressed={opcion === valor}
          onClick={() => onCambiar(opcion)}
        >
          <Icono size={16} aria-hidden="true" />
        </button>
      ))}
    </div>
  );
}
