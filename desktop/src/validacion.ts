import { z } from "zod";

/**
 * Reglas de cédula/nombre compartidas por los formularios que las editan
 * (Contratista, Usuario) — estaban duplicadas byte a byte, esquema de zod y
 * sanitizador de input incluidos. La validación real y definitiva vive en
 * el núcleo (`services/*_service.rs`); esto es sólo feedback inmediato sin
 * ida y vuelta al backend.
 */

export const cedulaSchema = z
  .string()
  .min(1, "La cédula es obligatoria")
  .regex(/^\d+$/, "La cédula sólo puede tener números");

export const nombreSchema = z
  .string()
  .min(1, "El nombre es obligatorio")
  .regex(/^[\p{L}\s'-]+$/u, "El nombre no puede tener números ni símbolos");

/** Sanitiza en vivo mientras se escribe (`onChange`) — deja sólo dígitos.
 * Complementa a `cedulaSchema`, no lo reemplaza: la regex sigue validando
 * al enviar, por si el valor llega de otro lado (`defaultValues`, pegar
 * texto de una fuente que no pasó por este `onChange`). */
export function sanearSoloDigitos(valor: string): string {
  return valor.replace(/\D/g, "");
}

/** Igual que `sanearSoloDigitos`, para el campo nombre. */
export function sanearSoloLetras(valor: string): string {
  return valor.replace(/[^\p{L}\s'-]/gu, "");
}
