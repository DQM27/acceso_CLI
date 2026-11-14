import type { ICellRendererParams } from "ag-grid-community";
import { Monitor, Smartphone } from "lucide-react";
import { textoDispositivo } from "../pantallas/Historial.logica";

/** Celda "Dispositivo" de Historial: el ícono, con el nombre al pasar el mouse
 * y para el lector de pantalla. Antes era un emoji, que se veía distinto al
 * resto de los íconos (auditoría 2026-11-14, igual que el escritorio). Un tipo
 * desconocido se muestra como texto. */
export default function IconoDispositivo({ value }: ICellRendererParams) {
  const tipo = typeof value === "string" ? value : null;
  const nombre = textoDispositivo(tipo);
  const Icono = tipo === "pc" ? Monitor : tipo === "mobile" ? Smartphone : null;
  if (!Icono) return nombre;
  return (
    <span title={nombre} aria-label={nombre} style={{ display: "inline-flex", verticalAlign: "middle" }}>
      <Icono size={16} aria-hidden="true" />
    </span>
  );
}
