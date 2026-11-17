import { solicitarSincronizacionNube } from "../eventosNube";
import { invoke } from "./invocar";

// Espejo de comandos/gafetes.rs y dto/gafetes.rs.

/** `Perdido`: no se puede asignar (no está físicamente), con o sin
 * responsable. */
export type EstadoGafete = "Disponible" | "Perdido";

/** Pool físico al que pertenece el gafete — espejo de `TipoGafete` (Rust). */
export type TipoGafete = "Contratista" | "Visita" | "ProvisionalKof" | "Proveedor";

export type MotivoResolucionGafete = "Pagado" | "Aparecido";

/** Espejo de `application::ClaseTenedorGafete`. Los valores coinciden con los
 * de `ClaseActivo` (`activosUnificados.ts`). */
export type ClaseTenedorGafete = "contratista" | "proveedor" | "correo" | "kof";

/** Dónde vive el registro: en este equipo (`id`) o en el otro dispositivo del
 * sitio (`uuid`). Espejo de `application::OrigenTenedorGafete`. */
export type OrigenTenedorGafete = { tipo: "local"; id: number } | { tipo: "remoto"; uuid: string };

/** Espejo de `application::TenedorGafete`: uno de los que tienen abierto un
 * gafete. `detalle` es la empresa (contratista, proveedor), el motivo (correo)
 * o el código de empleado (KOF). */
export interface TenedorGafete {
  clase: ClaseTenedorGafete;
  origen: OrigenTenedorGafete;
  nombre: string;
  detalle: string | null;
}

/** Quién tiene abierto el gafete `numero`, de todas las clases a la vez. Más de
 * uno es normal: cada conjunto de gafetes numera por su cuenta, así que el 1 de
 * un proveedor y el 1 de un contratista son dos gafetes distintos. La respuesta
 * la da el núcleo; acá no se decide nada. */
export function tenedoresDelGafete(numero: number): Promise<TenedorGafete[]> {
  return invoke("tenedores_del_gafete", { numero });
}

export interface GafeteResumen {
  id: number;
  numero: number;
  tipo: TipoGafete;
  estado: EstadoGafete;
  contratista_portador_id: number | null;
  contratista_portador_nombre: string | null;
  visita_portador_id: number | null;
  visita_portador_nombre: string | null;
  proveedor_portador_id: number | null;
  proveedor_portador_nombre: string | null;
  encargado_portador_id: number | null;
  encargado_portador_nombre: string | null;
  fecha_marcado_perdido: string | null;
}

/** Quién responde por este gafete perdido, sea cual sea su tipo — a lo sumo
 * uno de estos campos tiene valor (el `CHECK` de `gafetes` lo garantiza).
 * `null` si está disponible, si no se sabe quién lo tenía o si ya pagó. */
export function nombrePortador(
  fila: Pick<
    GafeteResumen,
    | "contratista_portador_nombre"
    | "visita_portador_nombre"
    | "proveedor_portador_nombre"
    | "encargado_portador_nombre"
  >,
): string | null {
  return (
    fila.contratista_portador_nombre ??
    fila.visita_portador_nombre ??
    fila.proveedor_portador_nombre ??
    fila.encargado_portador_nombre
  );
}

// snake_case a propósito — espejo exacto de `TipoGafeteEntrada` (Rust,
// `#[serde(rename_all = "snake_case")]`).
export type TipoGafeteEntrada = "contratista" | "visita" | "provisional_kof" | "proveedor";

export interface FiltroGafetes {
  numero?: number;
  tipo?: TipoGafeteEntrada;
  // snake_case y capitalizado a propósito — espejo exacto de
  // `EstadoGafeteEntrada` (Rust, `#[serde(rename_all = "snake_case")]`).
  estado?: "disponible" | "perdido";
}

/** `Resuelto`: cierre anterior a la versión con pago y recuperación
 * separados (sólo aparece en el historial viejo). */
export type TipoIncidenteGafete = "Perdido" | "Pagado" | "Recuperado" | "Resuelto";

/** Un reporte del historial de un gafete — espejo de `IncidenteGafete`
 * (`src/database/queries/gafetes_incidentes.rs`). */
export interface IncidenteGafete {
  id: number;
  tipo: TipoIncidenteGafete;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora: string;
  /** Quién hizo el reporte. */
  usuario_nombre: string;
  /** La persona marcada (`Perdido`) o a la que se le quitó la marca
   * (`Pagado`). */
  responsable_nombre: string | null;
  /** Texto libre de quien reportó. */
  detalle: string | null;
  /** Sólo en los `Resuelto` viejos. */
  motivo_resolucion: MotivoResolucionGafete | null;
  /** A qué gafete pertenece — no hace falta cuando ya se sabe por contexto
   * (`historialGafete`), pero es indispensable en la vista global de
   * Auditoría (`listarAuditoriaGafetes`, `../pantallas/Auditoria.tsx`). */
  gafete_numero: number;
}

export function buscarGafetes(filtro: FiltroGafetes): Promise<GafeteResumen[]> {
  return invoke("buscar_gafetes", { filtro });
}

export function historialGafete(id: number): Promise<IncidenteGafete[]> {
  return invoke("historial_gafete", { id });
}

// Cada cambio del catálogo pide subir enseguida (`solicitarSincronizacionNube`),
// igual que contratistas o ingresos: sin esto, el otro equipo del sitio no se
// enteraba hasta la sincronización periódica o el botón "Sincronizar".

export async function crearGafete(numero: number, tipo: TipoGafeteEntrada): Promise<number> {
  const id = await invoke<number>("crear_gafete", { numero, tipo });
  solicitarSincronizacionNube();
  return id;
}

export async function crearGafetesRango(
  desde: number,
  hasta: number,
  tipo: TipoGafeteEntrada,
): Promise<number[]> {
  const ids = await invoke<number[]>("crear_gafetes_rango", { desde, hasta, tipo });
  solicitarSincronizacionNube();
  return ids;
}

/** A quién se le asignó el gafete perdido, si se sabe. Espejo de
 * `ResponsableGafeteEntrada` (comandos/gafetes.rs). */
export type ResponsableGafete =
  | { tipo: "contratista"; id: number }
  | { tipo: "visita"; id: number }
  | { tipo: "provisional_kof"; id: number };

/** Sin `responsable`, el `detalle` es obligatorio. */
export async function reportarGafetePerdido(
  id: number,
  responsable: ResponsableGafete | null,
  detalle: string | null,
): Promise<void> {
  await invoke("reportar_gafete_perdido", { id, responsable, detalle });
  solicitarSincronizacionNube();
}

/** El responsable pagó: se le quita la marca; el gafete sigue perdido. */
export async function registrarPagoGafete(id: number, detalle: string | null): Promise<void> {
  await invoke("registrar_pago_gafete", { id, detalle });
  solicitarSincronizacionNube();
}

/** Apareció o se reimprimió: vuelve a estar disponible. */
export async function recuperarGafete(id: number, detalle: string): Promise<void> {
  await invoke("recuperar_gafete", { id, detalle });
  solicitarSincronizacionNube();
}

/** Qué es un número escrito en el conteo físico. Espejo de
 * `domain::gafete::LecturaConteo`. */
export type LecturaConteo =
  | { tipo: "esperado" }
  | { tipo: "no_existe" }
  | { tipo: "perdido" }
  | { tipo: "en_uso"; nombre: string };

export interface GafeteEnUso {
  numero: number;
  nombre: string;
}

/** Lo que dice "Comprobar". Espejo de `domain::gafete::ResultadoConteo`. */
export interface ResultadoConteo {
  esperados: number;
  presentes: number[];
  faltantes: number[];
  en_uso: GafeteEnUso[];
  en_uso_contados: GafeteEnUso[];
  no_existen: number[];
  perdidos_contados: number[];
  perdidos: number[];
  posibles_cruces: PosibleCruce[];
}

/** Por qué dos números se confunden fácil. Espejo de `domain::gafete::Parecido`. */
export type ParecidoGafete = "un_digito" | "invertido" | "seis_nueve";

/** Posible error de digitación al entregar: `contado` figura asignado y se
 * contó; `faltante` es probablemente el que esa persona tiene puesto. Espejo
 * de `domain::gafete::PosibleCruce`. */
export interface PosibleCruce {
  contado: GafeteEnUso;
  faltante: number;
  parecido: ParecidoGafete;
}

export function leerNumeroConteoGafetes(
  tipo: TipoGafeteEntrada,
  numero: number,
): Promise<LecturaConteo> {
  return invoke("leer_numero_conteo_gafetes", { tipo, numero });
}

export function comprobarConteoGafetes(
  tipo: TipoGafeteEntrada,
  numeros: number[],
): Promise<ResultadoConteo> {
  return invoke("comprobar_conteo_gafetes", { tipo, numeros });
}
