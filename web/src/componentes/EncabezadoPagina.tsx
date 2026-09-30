import type { ReactNode } from "react";

interface EncabezadoPaginaProps {
  titulo: string;
  descripcion?: string;
  accion?: ReactNode;
}

/** Encabezado fijo para cada pantalla: título, una línea de descripción y la
 * acción principal a la derecha (ver auditoria-panel-web, principio 1). */
export default function EncabezadoPagina({ titulo, descripcion, accion }: EncabezadoPaginaProps) {
  return (
    <header className="flex items-start justify-between gap-4 border-b border-borde px-6 py-4">
      <div>
        <h1 className="text-lg font-semibold text-texto">{titulo}</h1>
        {descripcion && <p className="mt-1 text-sm text-muted">{descripcion}</p>}
      </div>
      {accion && <div className="shrink-0">{accion}</div>}
    </header>
  );
}
