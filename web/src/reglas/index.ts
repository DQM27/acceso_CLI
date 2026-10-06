import { useEffect, useSyncExternalStore } from "react";
import iniciarWasm, { initSync } from "./wasm/reglas";
import * as wasm from "./wasm/reglas";
import type { TipoIngreso } from "../api/contratistas";

/**
 * Reglas de negocio del núcleo (crate `reglas/` en Rust), compiladas a
 * WebAssembly: el panel usa el MISMO código que escritorio, teléfono y la
 * Edge Function `admin-crear-contratista`, en vez de repetir las reglas en
 * TypeScript (ver docs/arquitectura/reglas-compartidas.md).
 *
 * Esto sirve para avisar mientras se llena el formulario. La decisión de
 * guardar la toma el servidor (la Edge Function valida otra vez con este
 * mismo paquete): lo que corre en el navegador se puede saltar.
 *
 * El paquete empieza a cargarse al arrancar (`iniciarReglas` en `main.tsx`),
 * sin frenar la primera pantalla: sólo lo necesitan los formularios, que
 * esperan con `useEstadoReglas` si todavía no llegó. Se regenera con
 * `scripts/generar-reglas-wasm.sh`.
 */

export type EstadoReglas = "cargando" | "lista" | "error";

let cargando: Promise<void> | null = null;
let estado: EstadoReglas = "cargando";
const oyentes = new Set<() => void>();

function cambiarEstado(nuevo: EstadoReglas) {
  estado = nuevo;
  oyentes.forEach((avisar) => avisar());
}

/** Carga el módulo WebAssembly (una sola vez). */
export function iniciarReglas(): Promise<void> {
  cargando ??= iniciarWasm().then(
    () => cambiarEstado("lista"),
    (error: unknown) => {
      cambiarEstado("error");
      throw error;
    },
  );
  return cargando;
}

/** Carga el módulo desde sus bytes, sin `fetch` (tests en Node). */
export function iniciarReglasDesdeBytes(bytes: BufferSource): void {
  initSync({ module: bytes });
  cargando = Promise.resolve();
  cambiarEstado("lista");
}

/** Estado de carga de las reglas, para que un formulario espere a que estén
 * (normalmente ya cargaron cuando alguien abre uno). Si nadie las pidió
 * todavía, empieza a cargarlas. */
export function useEstadoReglas(): EstadoReglas {
  useEffect(() => {
    iniciarReglas().catch(() => {
      // El error queda en el estado ("error"); main.tsx ya lo registra.
    });
  }, []);
  return useSyncExternalStore(
    (avisar) => {
      oyentes.add(avisar);
      return () => oyentes.delete(avisar);
    },
    () => estado,
  );
}

/** Datos del formulario de alta, con los códigos de la nube. */
export interface DatosContratistaReglas {
  cedula: string;
  nombre: string;
  tipo_ingreso: TipoIngreso;
  /** "AAAA-MM-DD", o null si no se cargó. */
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
  tipo_ingreso: TipoIngreso;
  es_personal_ruta: boolean;
  fecha_vencimiento_praind: string | null;
}

export type ResultadoValidacion =
  | { ok: true; contratista: ContratistaValido }
  | { ok: false; codigo: string; mensaje: string };

export const reglas = {
  /** ¿Este tipo (o ser personal de ruta) exige fecha de PRAIND? */
  requierePraind: (tipo: TipoIngreso, personalRuta = false): boolean =>
    wasm.requierePraind(tipo, personalRuta),
  /** ¿Este tipo exige gafete al entrar? */
  requiereGafete: (tipo: TipoIngreso, personalRuta = false): boolean =>
    wasm.requiereGafete(tipo, personalRuta),
  /** ¿Este tipo admite la casilla "personal de ruta"? */
  admitePersonalRuta: (tipo: TipoIngreso): boolean => wasm.admitePersonalRuta(tipo),
  /** Tipos que se pueden elegir para un contratista nuevo, en orden. */
  tiposIngresoSeleccionables: (): TipoIngreso[] => wasm.tiposIngresoSeleccionables() as TipoIngreso[],
  /** Cédula en su forma única, o undefined si no es de contratista. */
  normalizarCedula: (texto: string): string | undefined => wasm.normalizarCedulaContratista(texto),
  /** Todas las reglas de criterio de un contratista. `hoy`: "AAAA-MM-DD".
   * `anterior`: sólo al editar, lo que tenía guardado. */
  validarContratista: (
    datos: DatosContratistaReglas,
    hoy: string,
    anterior?: EstadoAnteriorContratista,
  ): ResultadoValidacion => wasm.validarContratista(datos, hoy, anterior ?? null) as ResultadoValidacion,
  /** Huella de las fuentes con que se generó el paquete (ver reglas.test.ts). */
  huellaFuentes: (): string => wasm.huellaFuentes(),
};
