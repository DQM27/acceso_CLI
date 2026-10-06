import { useEffect, useSyncExternalStore } from "react";
import iniciarWasm, { initSync } from "./wasm/reglas";
import * as wasm from "./wasm/reglas";

/**
 * Reglas de negocio del núcleo (crate `reglas/` en Rust), compiladas a
 * WebAssembly: la web de visitas usa el MISMO código que escritorio,
 * teléfono y el panel, en vez de repetir las reglas en TypeScript (ver
 * docs/arquitectura/reglas-compartidas.md). Por ejemplo, la cédula sale en
 * la misma forma única que reconoce el check-in de la portería.
 *
 * Sirve para avisar mientras se llena el formulario. La decisión de guardar
 * la toma la base (`crear_cita_anfitrion`), que revisa lo mismo: lo que
 * corre en el navegador se puede saltar.
 *
 * El paquete empieza a cargarse al arrancar (`iniciarReglas` en
 * `main.tsx`) sin frenar la primera pantalla; el formulario espera con
 * `useEstadoReglas` si todavía no llegó. Lo genera
 * `scripts/generar-reglas-wasm.sh` (misma copia que el panel).
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

export function reglasListas(): boolean {
  return estado === "lista";
}

/** Estado de carga, para que el formulario espere a que estén. Si nadie las
 * pidió todavía, empieza a cargarlas. */
export function useEstadoReglas(): EstadoReglas {
  useEffect(() => {
    iniciarReglas().catch(() => {
      // El error queda en el estado ("error"); el formulario lo muestra.
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

/** Lo que manda el formulario, tal cual. */
export interface EntradaCitaReglas {
  fecha_desde: string;
  fecha_hasta: string;
  hora_estimada?: string | null;
  motivo?: string | null;
  sitios: string[];
  visitantes: { nombre: string; cedula: string; empresa?: string | null; placa_vehiculo?: string | null }[];
}

/** La cita normalizada por el núcleo, lista para la base. */
export interface CitaValidaReglas {
  fecha_desde: string;
  fecha_hasta: string;
  /** "HH:MM" o null. */
  hora_estimada: string | null;
  motivo: string | null;
  sitios: string[];
  visitantes: { nombre: string; cedula: string; empresa: string | null; placa_vehiculo: string | null }[];
}

export type ResultadoCitaReglas =
  | { ok: true; cita: CitaValidaReglas }
  | { ok: false; errores: { campo: string; mensaje: string }[] };

function exigirListas() {
  if (estado !== "lista") throw new Error("Las reglas todavía se están cargando. Intente de nuevo en un momento.");
}

export const reglas = {
  /** Todas las reglas de una cita nueva. `hoy`: "AAAA-MM-DD" de Costa Rica. */
  validarCita: (datos: EntradaCitaReglas, hoy: string): ResultadoCitaReglas => {
    exigirListas();
    return wasm.validarCita(datos, hoy) as ResultadoCitaReglas;
  },
  /** Documento de un visitante en su forma única, o undefined si no es
   * válido (o si las reglas todavía no cargaron). */
  normalizarDocumento: (texto: string): string | undefined =>
    estado === "lista" ? wasm.normalizarDocumentoVisitante(texto) : undefined,
  /** Mientras se escribe un nombre de persona o empresa: en mayúscula, como
   * lo va a guardar el núcleo (regla `nombre`). Si las reglas todavía no
   * cargaron, queda tal cual: igual se guarda en mayúscula. */
  nombreMientrasSeEscribe: (texto: string): string =>
    estado === "lista" ? wasm.nombreMientrasSeEscribe(texto) : texto,
  /** Huella de las fuentes con que se generó el paquete (ver reglas.test.ts). */
  huellaFuentes: (): string => wasm.huellaFuentes(),
};
