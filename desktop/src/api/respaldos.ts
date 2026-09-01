import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/respaldos.rs y src/database/backup.rs del núcleo. Sólo
// respaldos "Manual" se crean desde acá — automático/pre-migración/
// pre-restauración los dispara el propio sistema, "por flag" es exclusivo
// de la CLI (--reset-root).

export type TipoRespaldo = "Manual" | "Automatico" | "PreMigracion" | "PreRestauracion" | "PorFlag";

export function etiquetaTipoRespaldo(tipo: TipoRespaldo): string {
  switch (tipo) {
    case "Manual":
      return "Manual";
    case "Automatico":
      return "Automático";
    case "PreMigracion":
      return "Pre-migración";
    case "PreRestauracion":
      return "Pre-restauración";
    case "PorFlag":
      return "Por flag (CLI)";
  }
}

export interface RespaldoResumen {
  ruta: string;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  creado_en: string;
  tipo: TipoRespaldo;
  tamano_bytes: number;
}

/** Rust serializa un enum con variantes mixtas (unitarias + con datos) así:
 * las unitarias como string plano, las que traen datos como objeto de una
 * sola clave (ver `ResultadoAcceso` en `api/ingresos.ts` para el mismo
 * criterio) — acá las tres variantes traen datos, así que no queda ninguna
 * unitaria en la unión. */
export type ResultadoValidacion =
  | { Valido: { version_esquema: number } }
  | { Invalido: string }
  | { EsquemaIncompatible: { version_encontrada: number } };

export function esValido(resultado: ResultadoValidacion): boolean {
  return "Valido" in resultado;
}

/** Mismo texto que `impl Display for ResultadoValidacion` (núcleo) —
 * duplicado a propósito del lado de TypeScript porque la unión ya
 * distingue las variantes; llamar al núcleo por esto sería un viaje IPC
 * de más por una traducción de tres líneas. */
export function textoValidacion(resultado: ResultadoValidacion): string {
  if ("Valido" in resultado) return `Válido (esquema v${resultado.Valido.version_esquema})`;
  if ("Invalido" in resultado) {
    return `No pasó la verificación: ${resultado.Invalido}`;
  }
  return `Esquema futuro no reconocido (v${resultado.EsquemaIncompatible.version_encontrada})`;
}

export function crearRespaldo(): Promise<RespaldoResumen> {
  return invoke("crear_respaldo");
}

export function listarRespaldos(): Promise<RespaldoResumen[]> {
  return invoke("listar_respaldos");
}

export function validarRespaldo(ruta: string): Promise<ResultadoValidacion> {
  return invoke("validar_respaldo", { ruta });
}

export function exportarRespaldo(ruta: string, destino: string): Promise<void> {
  return invoke("exportar_respaldo", { ruta, destino });
}

/** Reemplaza la base activa por `ruta` — cierra la sesión del lado del
 * núcleo al terminar (éxito o error), así que quien llama debe volver a
 * Login inmediatamente después, sin esperar otra señal (ver `Respaldos.tsx`). */
export function restaurarRespaldo(ruta: string): Promise<void> {
  return invoke("restaurar_respaldo", { ruta });
}
