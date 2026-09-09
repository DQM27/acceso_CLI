import { invoke } from "@tauri-apps/api/core";
import type { MedioIngreso } from "./ingresos";
import type { TipoIngreso } from "./contratistas";

// Espejo de comandos/nube.rs. `guardarSecretoDispositivo`/
// `secretoDispositivoGuardado` son exclusivos de ROOT (el secreto identifica
// al dispositivo entero ante el receptor, ver App.tsx/Nube.tsx). El resto
// (sincronizar, listar, cerrar) es de cualquier rol activo -- uso diario
// normal, no administración (ver `Operacion::UsarNube` en
// `src/domain/autorizacion.rs`) -- por eso el botón "Sincronizar" vive en la
// barra de estado (`BarraNube.tsx`), visible siempre, no sólo en esta
// pantalla.

export interface ResumenSincronizacion {
  enviados: number;
  fallidos: number;
  remotos_abiertos: number;
  cierres_recibidos: number;
  empresas_recibidas: number;
  contratistas_recibidos: number;
  gafetes_recibidos: number;
  movimientos_historial_recibidos: number;
  citas_recibidas: number;
  sitio_id: string;
  dispositivo_id: string;
  tipo: string;
  /** `true` si esta sincronización trajo la baja/desactivación de quien la
   * disparó -- el llamador debe cerrar la sesión local y volver al login
   * (ver `App.tsx`, donde ya se cerró del lado de Rust; esto es sólo para
   * que la UI reaccione). */
  sesion_expulsada: boolean;
}

export interface SesionRealtimeNube {
  base_url: string;
  apikey: string;
  access_token: string;
  expires_in: number;
  sitio_id: string;
  dispositivo_id: string;
  tipo: string;
  topic: string;
}

/** Ingreso abierto por el otro dispositivo del mismo sitio -- no vive en el
 * historial local (ver `database::schema`, tabla `ingresos_remotos`, y el
 * comentario en `nube::sincronizacion::IngresoRemoto`). */
export interface IngresoRemoto {
  uuid: string;
  contratista_nombre: string;
  /** ISO 8601 (UTC). */
  hora_entrada: string;
  usuario_entrada_nombre: string | null;
  contratista_cedula: string | null;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  medio_ingreso: string | null;
  gafete_numero: number | null;
}

/** La nube guarda `tipo_ingreso`/`medio_ingreso` en el formato que usa
 * Supabase (`PRAIND`, `IN_HOUSE`, `CAMINANDO`...), no en el `TipoIngreso`/
 * `MedioIngreso` que espera esta app (`Praind`, `InHouse`, `Caminando`) --
 * son dos serializaciones distintas del mismo dominio, nunca se
 * unificaron porque nunca se habían mostrado juntas hasta que Ingreso
 * Activo e Historial empezaron a fusionar filas locales con remotas.
 * Usado por `Activos.tsx` e `Historial.tsx`. */
export function tipoIngresoDesdeNube(valor: string | null): TipoIngreso | null {
  switch (valor) {
    case "PRAIND":
      return "Praind";
    case "IN_HOUSE":
      return "InHouse";
    case "POR_CORREO":
      return "PorCorreo";
    case "SWAT":
      return "Swat";
    default:
      return null;
  }
}

export function medioIngresoDesdeNube(valor: string | null): MedioIngreso | null {
  switch (valor) {
    case "CAMINANDO":
      return "Caminando";
    case "VEHICULO":
      return "Vehiculo";
    default:
      return null;
  }
}

/** Sólo tiene sentido con la base vacía (`requiereConfiguracionInicial`) --
 * ver `App.tsx`, pantalla "arranque". Guarda el secreto y trae el catálogo
 * remoto (usuarios incluidos) en el mismo paso, sin sesión: no hay con
 * quién loguearse todavía. */
export function configurarDispositivoInicial(secreto: string): Promise<ResumenSincronizacion> {
  return invoke("configurar_dispositivo_inicial", { secreto });
}

export function guardarSecretoDispositivo(secreto: string): Promise<void> {
  return invoke("guardar_secreto_dispositivo", { secreto });
}

export function secretoDispositivoGuardado(): Promise<boolean> {
  return invoke("secreto_dispositivo_guardado");
}

export function sincronizarConNube(): Promise<ResumenSincronizacion> {
  return invoke("sincronizar_con_nube");
}

export function sesionRealtimeNube(): Promise<SesionRealtimeNube> {
  return invoke("sesion_realtime_nube");
}

export function listarIngresosRemotos(): Promise<IngresoRemoto[]> {
  return invoke("listar_ingresos_remotos");
}

export function cerrarIngresoRemoto(uuid: string): Promise<void> {
  return invoke("cerrar_ingreso_remoto", { uuid });
}

/** Filas de la cola que ya agotaron los reintentos automáticos y quedaron
 * `fallido` de forma permanente -- necesitan que alguien las mire. */
export function fallosPermanentesNube(): Promise<number> {
  return invoke("fallos_permanentes_nube");
}
