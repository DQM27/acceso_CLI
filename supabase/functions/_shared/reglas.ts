// Reglas de negocio del núcleo (crate `reglas/` en Rust) compiladas a
// WebAssembly: las Edge Functions validan con el MISMO código que
// escritorio, teléfono y el panel web (ver docs/arquitectura/reglas-compartidas.md).
//
// El paquete lo genera scripts/generar-reglas-wasm.sh en ./reglas/; el .wasm
// viaja comprimido (gzip) y en base64, porque el despliegue de funciones sólo
// admite texto. Se carga una sola vez, al importar este módulo.

import {
  admitePersonalRuta,
  huellaFuentes,
  initSync,
  nombreEnMayusculas,
  normalizarCedulaContratista,
  requiereGafete,
  requierePraind,
  tiposIngresoSeleccionables,
  validarContratista as validarContratistaWasm,
} from "./reglas/reglas.js";
import { REGLAS_WASM_GZIP_BASE64 } from "./reglas/reglas_wasm_base64.ts";

function base64ABytes(base64: string): Uint8Array<ArrayBuffer> {
  const binario = atob(base64);
  const bytes = new Uint8Array(binario.length);
  for (let i = 0; i < binario.length; i++) bytes[i] = binario.charCodeAt(i);
  return bytes;
}

async function descomprimir(gzip: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> {
  const flujo = new Blob([gzip]).stream().pipeThrough(new DecompressionStream("gzip"));
  return new Uint8Array(await new Response(flujo).arrayBuffer());
}

initSync({ module: await descomprimir(base64ABytes(REGLAS_WASM_GZIP_BASE64)) });

export type TipoIngreso = "PRAIND" | "IN_HOUSE" | "POR_CORREO" | "SWAT";

export interface DatosContratistaReglas {
  cedula: string;
  nombre: string;
  tipo_ingreso: string;
  /** "AAAA-MM-DD", o null. */
  fecha_vencimiento_praind: string | null;
  es_personal_ruta?: boolean;
  tiene_acceso?: boolean;
}

export interface ContratistaValido {
  cedula: string;
  nombre: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string | null;
  es_personal_ruta: boolean;
  tiene_acceso: boolean;
}

/** Al editar: lo que el contratista tenía guardado. */
export interface EstadoAnteriorContratista {
  tipo_ingreso: string;
  es_personal_ruta: boolean;
  fecha_vencimiento_praind: string | null;
}

export type ResultadoValidacion =
  | { ok: true; contratista: ContratistaValido }
  | { ok: false; codigo: string; mensaje: string };

/** Todas las reglas de criterio de un contratista. `hoy`: "AAAA-MM-DD" en
 * Costa Rica. `anterior`: sólo al editar, lo que tenía guardado. */
export function validarContratista(
  datos: DatosContratistaReglas,
  hoy: string,
  anterior?: EstadoAnteriorContratista,
): ResultadoValidacion {
  return validarContratistaWasm(datos, hoy, anterior ?? null) as ResultadoValidacion;
}

/** La fecha de hoy en Costa Rica, "AAAA-MM-DD" (la que decide si un PRAIND
 * está vencido, igual que en las apps). */
export function hoyCostaRica(ahora: Date = new Date()): string {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Costa_Rica",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(ahora);
}

export {
  admitePersonalRuta,
  huellaFuentes,
  nombreEnMayusculas,
  normalizarCedulaContratista,
  requiereGafete,
  requierePraind,
  tiposIngresoSeleccionables,
};
