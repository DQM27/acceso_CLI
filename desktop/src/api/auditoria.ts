import { invoke } from "@tauri-apps/api/core";
import type { IncidenteGafete } from "./gafetes";
import type { CargaCompleta } from "./comun";

// Espejo de comandos/auditoria.rs y
// src/database/queries/auditoria.rs (`CambioAuditado`) del núcleo. Sin
// paginado a propósito — mismo criterio que Historial: trae todo de una vez
// y deja que AG Grid virtualice del lado del cliente, acotado por
// `CargaCompleta.truncado` (ver `api/comun.ts`) — a diferencia de
// Historial, esta pantalla no tiene selector de rango de fechas. Auditoría
// genérica (contratistas, empresas, usuarios) desde 2026-08-28 — antes era
// sólo de contratistas.

export type EntidadAuditada = "Contratista" | "Empresa" | "Usuario";

export interface CambioAuditado {
  id: number;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora: string;
  usuario_id: number;
  usuario_nombre: string;
  entidad: EntidadAuditada;
  entidad_id: number;
  entidad_nombre: string;
  /** Clave cruda de columna (`"cedula"`, `"nombre"`, `"empresa_id"`,
   * `"tipo_ingreso"`, `"fecha_vencimiento_praind"`, `"es_personal_ruta"`,
   * `"tiene_acceso"`, `"rol"`, `"activo"`, `"password"`) — ver
   * `etiquetaCampo`. */
  campo: string;
  valor_anterior: string | null;
  valor_nuevo: string | null;
}

export function etiquetaEntidad(entidad: EntidadAuditada): string {
  switch (entidad) {
    case "Contratista":
      return "Contratista";
    case "Empresa":
      return "Empresa";
    case "Usuario":
      return "Usuario";
  }
}

/** Espejo de `descripcion_cambio`/`valor_presentable`
 * (`src/comandos/render/auditoria.rs`) — mismas etiquetas, para que la GUI
 * no invente una traducción distinta de las mismas claves crudas. */
export function etiquetaCampo(campo: string): string {
  switch (campo) {
    case "cedula":
      return "Cédula";
    case "nombre":
      return "Nombre";
    case "empresa_id":
      return "Empresa";
    case "tipo_ingreso":
      return "Tipo de ingreso";
    case "fecha_vencimiento_praind":
      return "Vencimiento PRAIND";
    case "es_personal_ruta":
      return "Personal de ruta";
    case "tiene_acceso":
      return "Acceso";
    case "rol":
      return "Rol";
    case "activo":
      return "Activo";
    case "password":
      return "Contraseña";
    default:
      return campo;
  }
}

/** `password` es un marcador de evento sin valores (ver
 * `UsuarioService::cambiar_password_con_hash_auditado` en el núcleo) — sólo
 * importa que ocurrió y cuándo, no hay antes/después que mostrar. */
export function valorPresentable(campo: string, valor: string | null): string {
  if (campo === "password") return "—";
  if (valor === null) {
    return campo === "fecha_vencimiento_praind" ? "Sin fecha" : "—";
  }
  if (campo === "tipo_ingreso" && valor === "IN_HOUSE") return "IN HOUSE";
  if (campo === "tipo_ingreso" && valor === "POR_CORREO") return "POR CORREO";
  if (campo === "fecha_vencimiento_praind") {
    // "YYYY-MM-DD" → "DD/MM/YYYY", como texto — sin pasar por `Date` (mismo
    // motivo que el resto de la app: evita corrimientos de huso horario).
    const partes = valor.split("-");
    if (partes.length === 3) {
      const [anio, mes, dia] = partes;
      return `${dia}/${mes}/${anio}`;
    }
    return valor;
  }
  if (campo === "tiene_acceso" && valor === "HABILITADO") return "Habilitado";
  if (campo === "tiene_acceso" && valor === "DESHABILITADO") return "Deshabilitado";
  if ((campo === "es_personal_ruta" || campo === "activo") && valor === "SI") return "Sí";
  if ((campo === "es_personal_ruta" || campo === "activo") && valor === "NO") return "No";
  return valor;
}

export function listarAuditoria(): Promise<CargaCompleta<CambioAuditado>> {
  return invoke("listar_auditoria");
}

/** Incidentes de gafetes (marcar perdido/resolver) — misma pantalla de
 * Auditoría, tabla de origen distinta (`gafetes_incidentes`, no
 * `auditoria_cambios`), mismo gate (`Operacion::VerAuditoria`). */
export function listarAuditoriaGafetes(): Promise<IncidenteGafete[]> {
  return invoke("listar_auditoria_gafetes");
}
