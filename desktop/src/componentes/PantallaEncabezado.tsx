import type { ReactNode } from "react";

/**
 * Encabezado uniforme de pantalla — título a la izquierda, acciones (botón
 * "+ Nuevo…", etc.) a la derecha. Reemplaza al header improvisado que cada
 * pantalla armaba por su cuenta con estilos en línea propios.
 */
export default function PantallaEncabezado({
  titulo,
  acciones,
}: {
  titulo: string;
  acciones?: ReactNode;
}) {
  return (
    <div className="pantalla-encabezado">
      <h1>{titulo}</h1>
      {acciones && <div style={{ display: "flex", gap: "0.5rem" }}>{acciones}</div>}
    </div>
  );
}
