import { ChevronLeft, ChevronRight } from "lucide-react";

/** Rango "1–100 de 4.321" de la página `pagina` (base 0). Vacío: "Sin resultados". */
export function textoRangoPagina(pagina: number, tamano: number, total: number): string {
  if (total === 0) return "Sin resultados";
  const primera = pagina * tamano + 1;
  const ultima = Math.min(total, (pagina + 1) * tamano);
  return `${primera.toLocaleString("es-CR")}–${ultima.toLocaleString("es-CR")} de ${total.toLocaleString("es-CR")}`;
}

/** Cantidad de páginas (mínimo 1, aunque no haya resultados). */
export function totalPaginas(tamano: number, total: number): number {
  return Math.max(1, Math.ceil(total / tamano));
}

interface PaginadorProps {
  /** Base 0. */
  pagina: number;
  tamano: number;
  total: number;
  cargando?: boolean;
  onCambiar: (pagina: number) => void;
}

/** Barra anterior/siguiente de una tabla paginada en el servidor. */
export default function Paginador({ pagina, tamano, total, cargando, onCambiar }: PaginadorProps) {
  const paginas = totalPaginas(tamano, total);
  return (
    <div className="flex items-center justify-end gap-2 pt-2 text-[0.85rem] text-muted" aria-live="polite">
      <span>{cargando ? "Cargando…" : textoRangoPagina(pagina, tamano, total)}</span>
      <button
        type="button"
        className="boton boton-icono"
        title="Página anterior"
        aria-label="Página anterior"
        disabled={pagina <= 0}
        onClick={() => onCambiar(pagina - 1)}
      >
        <ChevronLeft size={16} />
      </button>
      <span>
        Página {(pagina + 1).toLocaleString("es-CR")} de {paginas.toLocaleString("es-CR")}
      </span>
      <button
        type="button"
        className="boton boton-icono"
        title="Página siguiente"
        aria-label="Página siguiente"
        disabled={pagina >= paginas - 1}
        onClick={() => onCambiar(pagina + 1)}
      >
        <ChevronRight size={16} />
      </button>
    </div>
  );
}
