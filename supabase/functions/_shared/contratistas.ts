// Lo común al alta y a la edición de contratistas desde el panel
// (admin-crear-contratista y admin-editar-contratista): validación del id y
// los textos de lo que no es una regla de criterio, iguales a los de las apps
// (`mensajes::mensaje_contratista` del núcleo).

import type { ContratistaValido } from "./reglas.ts";

export const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const MENSAJE_EMPRESA_NO_ENCONTRADA = "La empresa seleccionada ya no existe";
export const MENSAJE_CEDULA_DUPLICADA = "Ya existe un contratista con esa cédula";
export const MENSAJE_CONTRATISTA_NO_ENCONTRADO = "El contratista ya no existe";
export const MENSAJE_CEDULA_CON_INGRESO_ACTIVO =
  "No se puede cambiar la cédula mientras está adentro — registre primero la salida";

export interface EmpresaEncontrada {
  id: string;
  nombre: string;
}

/** Resultado de guardar: la fila, o por qué no se pudo. */
export type ResultadoGuardar =
  | { ok: true; fila: Record<string, unknown> }
  | { ok: false; motivo: "cedula_duplicada" | "error"; detalle?: string };

/** Columnas de `contratistas` que salen de los datos validados. */
export function columnasContratista(contratista: ContratistaValido, empresa: EmpresaEncontrada) {
  return {
    nombre: contratista.nombre,
    identificacion: contratista.cedula,
    activo: contratista.tiene_acceso,
    empresa_id: empresa.id,
    empresa_nombre: empresa.nombre,
    tipo_ingreso: contratista.tipo_ingreso,
    fecha_vencimiento_praind: contratista.fecha_vencimiento_praind,
    es_personal_ruta: contratista.es_personal_ruta,
  };
}

/** Lee el texto de un campo del cuerpo ("" si no es texto). */
export function texto(valor: unknown): string {
  return typeof valor === "string" ? valor : "";
}
