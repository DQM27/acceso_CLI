import { invoke } from "@tauri-apps/api/core";
import type { TipoIngreso } from "./contratistas";

// Espejo de comandos/ingresos.rs — ver también src/services/registro_ingreso_service.rs
// y src/domain/resultado_acceso.rs del núcleo.

export type MedioIngreso = "Caminando" | "Vehiculo";
export const MEDIOS_INGRESO: MedioIngreso[] = ["Caminando", "Vehiculo"];

export type MotivoDenegacion =
  | "SinAcceso"
  | "PraindVencido"
  | "PraindNoRegistrado"
  | "EmpresaInactiva";

/// Rust serializa un enum con variantes mixtas (unitarias + con datos) así:
/// las unitarias como string plano, la que trae datos como objeto de una
/// sola clave. `ResultadoAcceso::Denegado(MotivoDenegacion)` cae en el
/// segundo caso — de ahí la unión en vez de un string único.
export type ResultadoAcceso =
  | "Permitido"
  | "PermitidoConAdvertencia"
  | { Denegado: MotivoDenegacion };

export interface PreparacionIngreso {
  contratista_id: number;
  cedula: string;
  nombre: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  resultado_acceso: ResultadoAcceso;
  requiere_gafete: boolean;
  tiene_ingreso_activo: boolean;
}

export interface ResultadoRegistroEntrada {
  registro_id: number;
  resultado_acceso: ResultadoAcceso;
}

export type ResultadoIngresoRegistrado =
  | "Permitido"
  | "Migrado"
  | { PermitidoConAdvertencia: "PraindProximoVencer" | "DatosReconstruidos" };

export interface IngresoActivoResumen {
  registro_id: number;
  contratista_id: number;
  cedula: string;
  contratista_nombre: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  medio_ingreso: MedioIngreso;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_ingreso: string;
  gafete_numero: number | null;
  usuario_ingreso_nombre: string;
  resultado_registrado: ResultadoIngresoRegistrado;
  resultado_acceso: ResultadoAcceso;
}

export interface ListaIngresosActivosResumen {
  items: IngresoActivoResumen[];
  total: number;
}

/// Espejo de `puede_continuar`/`mensaje_bloqueo`
/// (`src/tui/nuevo_ingreso/state.rs`) — `preparar_ingreso` no rechaza estos
/// casos (devuelve `Ok` igual, con el motivo adentro), así que quien llama
/// decide si deja continuar. La validación real y definitiva la vuelve a
/// hacer el backend en `registrar_ingreso` de todos modos.
export function puedeContinuar(p: PreparacionIngreso): boolean {
  return !p.tiene_ingreso_activo && typeof p.resultado_acceso !== "object";
}

export function mensajeBloqueo(p: PreparacionIngreso): string {
  if (p.tiene_ingreso_activo) {
    return "El contratista ya tiene un ingreso activo.";
  }
  if (typeof p.resultado_acceso === "object") {
    return mensajeMotivoDenegacion(p.resultado_acceso.Denegado);
  }
  return "No se puede continuar con este contratista.";
}

export function mensajeMotivoDenegacion(motivo: MotivoDenegacion): string {
  switch (motivo) {
    case "SinAcceso":
      return "Acceso denegado · no tiene acceso autorizado";
    case "PraindVencido":
      return "Acceso denegado · PRAIND vencido";
    case "PraindNoRegistrado":
      return "Acceso denegado · PRAIND sin fecha registrada";
    case "EmpresaInactiva":
      return "Acceso denegado · la empresa está inactiva";
  }
}

const MAX_LARGO_GAFETES = 60;

/** Mismo criterio que `SalidaGafeteState::asignar_texto` (`--comandos`):
 * sólo dígitos, coma (separador de lista) y espacio. Compartido por
 * `SalidaModal` y la consola — ambos implementan el mismo modo enclavado
 * de "sacar por gafete", uno como panel de formulario, el otro como modo
 * de la línea de comandos. */
export function sanearGafetes(texto: string): string {
  return texto
    .split("")
    .filter((c) => /[\d,\s]/.test(c))
    .slice(0, MAX_LARGO_GAFETES)
    .join("");
}

export function gafetesDe(texto: string): number[] {
  return texto
    .split(",")
    .map((token) => token.trim())
    .filter((token) => token.length > 0)
    .map(Number)
    .filter((n) => Number.isInteger(n));
}

export function listarIngresosActivos(): Promise<ListaIngresosActivosResumen> {
  return invoke("listar_ingresos_activos");
}

export function prepararIngreso(contratistaId: number): Promise<PreparacionIngreso> {
  return invoke("preparar_ingreso", { contratistaId });
}

export function registrarIngreso(
  contratistaId: number,
  medio: MedioIngreso,
  gafete: number | null,
): Promise<ResultadoRegistroEntrada> {
  return invoke("registrar_ingreso", { contratistaId, medio, gafete });
}

export function registrarSalida(id: number): Promise<void> {
  return invoke("registrar_salida", { id });
}
