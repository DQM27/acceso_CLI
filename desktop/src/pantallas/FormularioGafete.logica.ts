import { z } from "zod";
import { BadgeCheck, Boxes, DoorOpen, HardHat } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { TipoGafeteEntrada } from "../api";
import { textoGafete } from "../busqueda";

export const numeroValido = (valor: string) => {
  const n = Number(valor);
  return valor.trim() !== "" && Number.isInteger(n) && n > 0;
};

// Validación de UI antes de despachar — la validación real y atómica queda
// en GafeteService (núcleo); esto sólo evita un típo evidente (vacío, no
// numérico) o un rango descomunal por error de tecleo (tope defensivo de
// 200, mismo criterio que la TUI — docs/plan-gafetes.md).
export const esquema = z
  .object({
    modo: z.enum(["individual", "rango"]),
    tipo: z.enum(["contratista", "visita", "provisional_kof", "proveedor"]),
    numero: z.string(),
    desde: z.string(),
    hasta: z.string(),
  })
  .superRefine((valores, ctx) => {
    if (valores.modo === "individual") {
      if (!numeroValido(valores.numero)) {
        ctx.addIssue({
          code: "custom",
          path: ["numero"],
          message: "Ingrese un número de gafete válido",
        });
      }
      return;
    }
    if (!numeroValido(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["desde"], message: 'Ingrese un "desde" válido' });
      return;
    }
    if (!numeroValido(valores.hasta) || Number(valores.hasta) < Number(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["hasta"], message: "El rango no es válido" });
      return;
    }
    if (Number(valores.hasta) - Number(valores.desde) > 200) {
      ctx.addIssue({
        code: "custom",
        path: ["hasta"],
        message: "El rango es demasiado grande (máximo 200 a la vez)",
      });
    }
  });

export interface ValoresFormulario {
  modo: "individual" | "rango";
  tipo: TipoGafeteEntrada;
  numero: string;
  desde: string;
  hasta: string;
}

/** Tipos en el orden en que se muestran, con el mismo ícono que su sección
 * en el menú lateral. */
export const TIPOS: { valor: TipoGafeteEntrada; etiqueta: string; singular: string; Icono: LucideIcon }[] = [
  { valor: "contratista", etiqueta: "Contratista", singular: "contratista", Icono: HardHat },
  { valor: "proveedor", etiqueta: "Proveedor", singular: "proveedor", Icono: Boxes },
  { valor: "provisional_kof", etiqueta: "KOF", singular: "provisional KOF", Icono: BadgeCheck },
  { valor: "visita", etiqueta: "Visita", singular: "visita", Icono: DoorOpen },
];

/** Qué se va a crear, en una frase, y el texto del botón -- `null` en la
 * frase mientras los números no alcancen para decirlo (vacíos o rango al
 * revés). */
export function resumenCreacion(valores: ValoresFormulario): {
  frase: string | null;
  boton: string;
} {
  const tipo = TIPOS.find((t) => t.valor === valores.tipo)?.singular ?? valores.tipo;
  if (valores.modo === "individual") {
    return {
      frase: numeroValido(valores.numero)
        ? `Se creará el gafete ${textoGafete(Number(valores.numero))} de ${tipo}.`
        : null,
      boton: "Crear gafete",
    };
  }
  const desde = Number(valores.desde);
  const hasta = Number(valores.hasta);
  if (!numeroValido(valores.desde) || !numeroValido(valores.hasta) || hasta < desde) {
    return { frase: null, boton: "Crear gafetes" };
  }
  const cantidad = hasta - desde + 1;
  if (cantidad === 1) {
    return { frase: `Se creará el gafete ${textoGafete(desde)} de ${tipo}.`, boton: "Crear gafete" };
  }
  return {
    frase: `Se crearán ${cantidad} gafetes de ${tipo}, del ${textoGafete(desde)} al ${textoGafete(hasta)}.`,
    boton: `Crear ${cantidad} gafetes`,
  };
}
