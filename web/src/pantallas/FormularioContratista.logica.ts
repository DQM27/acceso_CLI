import type { TipoIngreso } from "../api/contratistas";

/** Tipos de ingreso en el orden del formulario, con el texto que se lee en
 * pantalla ("IN HOUSE", no el valor interno "IN_HOUSE"). Sin "POR CORREO":
 * dejó de ser un tipo de contratista (2026-10-03); esas visitas se registran
 * como ingreso por correo en el escritorio y el teléfono. */
export const TIPOS_INGRESO: { valor: TipoIngreso; etiqueta: string }[] = [
  { valor: "PRAIND", etiqueta: "PRAIND" },
  { valor: "IN_HOUSE", etiqueta: "IN HOUSE" },
  { valor: "SWAT", etiqueta: "SWAT" },
];

/** Si el formulario muestra la fecha de vencimiento del PRAIND. Solo cuando la
 * persona va a tener acceso: a quien se crea bloqueado no se le pide, no va a
 * entrar de todos modos. La regla de fondo (PRAIND e IN HOUSE lo requieren, y
 * vigente) la aplica la base al guardar; esto solo decide qué mostrar. */
export function pidePraind(tipo: TipoIngreso, conAcceso: boolean): boolean {
  return conAcceso && (tipo === "PRAIND" || tipo === "IN_HOUSE");
}

/** Lo mínimo que se revisa antes de enviar; las reglas de verdad (cédula, nombre,
 * PRAIND, duplicados) las aplica la base y su mensaje se muestra tal cual. */
export function errorAntesDeEnviar(valores: { empresaId: string }): string | null {
  return valores.empresaId === "" ? "Elija la empresa" : null;
}
