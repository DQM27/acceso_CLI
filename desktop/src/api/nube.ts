import { invoke } from "./invocar";
import { solicitarSincronizacionNube } from "../eventosNube";
import type { MedioIngreso, NivelPermanencia } from "./ingresos";
import type { TipoIngreso } from "./contratistas";

// Espejo de comandos/nube.rs. Este equipo se vincula a la nube una sola vez,
// canjeando un código del panel en el arranque inicial
// (`vincularDispositivoInicial`). El resto (sincronizar, listar, cerrar) es de cualquier sesión
// activa -- uso diario normal, no administración -- por eso el botón
// "Sincronizar" vive en la barra de estado (`BarraNube.tsx`), visible
// siempre.

export interface ResumenSincronizacion {
  enviados: number;
  fallidos: number;
  remotos_abiertos: number;
  cierres_recibidos: number;
  /** Mismo criterio que `cierres_recibidos`, pero para ingresos de
   * proveedor. */
  cierres_recibidos_proveedor: number;
  /** Lo mismo para ingresos por correo. */
  cierres_recibidos_correo: number;
  empresas_recibidas: number;
  contratistas_recibidos: number;
  gafetes_recibidos: number;
  vehiculos_ruta_recibidos: number;
  encargados_ruta_recibidos: number;
  movimientos_historial_recibidos: number;
  citas_recibidas: number;
  historial_visitas_recibidos: number;
  /** Mismo criterio que `historial_visitas_recibidos`, pero para ingresos
   * de proveedor. */
  historial_ingresos_proveedor_recibidos: number;
  /** Lo mismo para ingresos por correo. */
  historial_ingresos_correo_recibidos: number;
  /** Mismo criterio que `historial_visitas_recibidos`, pero para préstamos
   * de gafete provisional KOF. */
  historial_gafetes_provisionales_recibidos: number;
  sitio_id: string;
  dispositivo_id: string;
  tipo: string;
  /** `true` si esta sincronización trajo la baja/desactivación de quien la
   * disparó -- el llamador debe cerrar la sesión local y volver al login
   * (ver `App.tsx`, donde ya se cerró del lado de Rust; esto es sólo para
   * que la UI reaccione). */
  sesion_expulsada: boolean;
  /** `true` si la sesión se cerró porque el usuario inició sesión en otra
   * unidad (sesión única por unidad); `sesion_expulsada` también es `true`.
   * Sólo cambia el aviso. */
  sesion_en_otra_unidad: boolean;
  /** Ingresos que quedaron activos en este dispositivo pero que la nube
   * dice que TAMBIÉN están activos en otro sitio (`docs/pendientes.md`,
   * "alertar luego al sincronizar") -- mejor esfuerzo, vacío si el chequeo
   * falla. */
  conflictos_ingreso: ConflictoIngresoActivo[];
  /** Mismo criterio que `conflictos_ingreso`, pero para movimientos de
   * visita -- un visitante que quedó activo en este dispositivo pero que
   * la nube dice que también está activo en otro sitio. */
  conflictos_movimiento_visita: ConflictoMovimientoVisitaActivo[];
  /** Mismo criterio que `conflictos_ingreso`, pero para ingresos de
   * proveedor -- una cédula que quedó activa en este dispositivo pero que
   * la nube dice que también está activa en otro sitio. */
  conflictos_ingreso_proveedor: ConflictoIngresoProveedorActivo[];
  /** Lo mismo para ingresos por correo (mismo tipo de aviso). */
  conflictos_ingreso_correo: ConflictoIngresoProveedorActivo[];
  /** Ingresos con gafete que ESTE dispositivo registró, pero cuyo envío a
   * la nube fue rechazado porque otro dispositivo del mismo sitio ya
   * tiene ese número activo -- a diferencia de `conflictos_ingreso`, se
   * calcula con datos locales dentro del mismo `drenar_cola`, sin una
   * consulta remota aparte. */
  conflictos_gafete: ConflictoGafeteActivo[];
}

export interface ConflictoIngresoActivo {
  cedula: string;
  contratista_nombre: string;
  sitio_conflicto: string;
}

export interface ConflictoMovimientoVisitaActivo {
  cedula: string;
  visitante_nombre: string;
  sitio_conflicto: string;
}

export interface ConflictoIngresoProveedorActivo {
  cedula: string;
  nombre: string;
  sitio_conflicto: string;
}

/** Qué movimiento entregó el gafete en conflicto (`nube::TipoMovimientoGafete`). */
export type TipoMovimientoGafete = "contratista" | "proveedor" | "provisional_kof" | "por_correo" | "visita";

export interface ConflictoGafeteActivo {
  tipo: TipoMovimientoGafete;
  /** Contratista, proveedor o encargado de ruta (KOF). */
  nombre: string;
  gafete_numero: number;
  /** Hora de entrada, o de entrega en KOF. */
  fecha_hora: string;
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
  placa: string | null;
  nivel_permanencia: NivelPermanencia;
}

/** Espejo de `IngresoRemoto`, pero para el ciclo de proveedores -- ver
 * `database::schema`, tabla `ingresos_proveedor_remotos`, y
 * `nube::sincronizacion::IngresoProveedorRemoto`. */
export interface IngresoProveedorRemoto {
  uuid: string;
  cedula: string;
  nombre: string;
  empresa_nombre: string;
  placa: string | null;
  gafete_numero: number;
  /** ISO 8601 (UTC). */
  hora_entrada: string;
  usuario_entrada_nombre: string;
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
 * ver `App.tsx`, pantalla "arranque". Canjea el código de vinculación del
 * panel y trae el catálogo remoto (usuarios incluidos) en el mismo paso,
 * sin sesión: no hay con quién loguearse todavía. */
export function vincularDispositivoInicial(codigo: string): Promise<ResumenSincronizacion> {
  return invoke("vincular_dispositivo_inicial", { codigo });
}

/** Base vacía de un equipo que ya está vinculado (su clave sigue guardada):
 * baja el catálogo de su unidad sin pedir código. Pasa después de que un
 * equipo reconectado en otra unidad borra la base de la anterior. */
export function prepararEquipoVinculado(): Promise<ResumenSincronizacion> {
  return invoke("preparar_equipo_vinculado");
}

/** `true` si el equipo ya tiene una clave aceptada por la nube. */
export function equipoVinculado(): Promise<boolean> {
  return invoke("equipo_vinculado");
}

/** `true` si la última respuesta con autoridad de la nube fue que este
 * equipo está retirado: es cuando se ofrece "Reconectar este equipo". Lo
 * decide el núcleo con la respuesta de `device-auth`. */
export function equipoRetirado(): Promise<boolean> {
  return invoke("equipo_retirado");
}

/** Resultado de reconectar un equipo retirado (ver `reconectar_equipo`). */
export type ResultadoReconexion =
  | { tipo: "Reconectado"; unidad: string }
  | { tipo: "NecesitaConfirmacion"; pendientes: number; unidad: string }
  | { tipo: "ReconectadoEnOtraUnidad"; unidad: string };

/** Reconecta este equipo con el código de reconexión del panel. Si va a
 * otra unidad y hay registros sin subir, la primera vez responde
 * `NecesitaConfirmacion` sin gastar el código; para seguir, se vuelve a
 * llamar con los `pendientesConfirmados` que se mostraron. */
export function reconectarEquipo(
  codigo: string,
  pendientesConfirmados?: number,
): Promise<ResultadoReconexion> {
  return invoke("reconectar_equipo", { codigo, pendientesConfirmados: pendientesConfirmados ?? null });
}

/** Tras un aviso `dispositivo_expulsado`: descarta el token cacheado, le
 * pregunta a `device-auth` y devuelve si la nube confirmó el retiro. */
export function descartarTokenNube(): Promise<boolean> {
  return invoke("descartar_token_nube");
}

export function sincronizarConNube(): Promise<ResumenSincronizacion> {
  return invoke("sincronizar_con_nube");
}

/** Guarda la fila que trae un aviso en vivo (`registro`) directo en la base
 * local, sin consultar a la nube (en el escritorio, también su línea del
 * Historial). `true` si la aplicó; `false` si el aviso no trae datos
 * (queda para `sincronizarCambiosNube`). */
export function aplicarCambioNube(cambio: unknown): Promise<boolean> {
  return invoke("aplicar_cambio_nube", { cambio });
}

/** Aplica la orden de telemetría que trae un aviso `telemetria_config`
 * (grupos, vencimiento y versión): el núcleo la guarda y la aplica sólo si es
 * para `dispositivoId` y más nueva que la vigente. `true` si cambió lo que se
 * mide. Nunca debe tumbar nada: un aviso perdido lo cubre la sincronización. */
export function aplicarOrdenTelemetria(dispositivoId: string, aviso: unknown): Promise<boolean> {
  return invoke("aplicar_orden_telemetria", { dispositivoId, aviso });
}

/** Sincronización de un aviso en vivo: sólo las etapas de las tablas que
 * cambiaron (`payload.table` de `cambio_nube`) -- ver
 * `nube::AlcanceSincronizacion` en el núcleo. Una tabla desconocida cae en
 * la sincronización completa del lado Rust. */
export function sincronizarCambiosNube(tablas: string[]): Promise<ResumenSincronizacion> {
  return invoke("sincronizar_cambios_nube", { tablas });
}

/** Sólo sube la bandeja de salida, sin bajar nada: lo que dispara un
 * ingreso/salida registrado en este equipo. */
export function enviarCambiosNube(): Promise<ResumenSincronizacion> {
  return invoke("enviar_cambios_nube");
}

export function sesionRealtimeNube(): Promise<SesionRealtimeNube> {
  return invoke("sesion_realtime_nube");
}

export function listarIngresosRemotos(): Promise<IngresoRemoto[]> {
  return invoke("listar_ingresos_remotos");
}

/** Cierra contra la nube y después pide sincronizar (mismo criterio que
 * `cerrarPrestamoGafeteProvisionalRemoto`). Sin esto el historial no se
 * enteraba de la salida hasta el próximo pulso o un "Sincronizar" a mano:
 * el aviso Realtime de este cambio trae el id de ESTE dispositivo y
 * `nubeRealtime.ts` lo descarta a propósito, así que nadie volvía a bajar
 * `historial_sitio` (bug real reportado por el usuario 2026-09-23). */
export async function cerrarIngresoRemoto(uuid: string): Promise<void> {
  await invoke("cerrar_ingreso_remoto", { uuid });
  solicitarSincronizacionNube();
}

/** Abre de antemano las conexiones con la nube que usa la verificación de un
 * ingreso. Los formularios de ingreso lo piden al abrirse: mientras se escribe el
 * nombre, el apretón de manos TLS ya se hizo, y la primera verificación no paga
 * 450-590 ms de conexión nueva (medido contra staging con el grupo frío; en
 * caliente son ~135 ms). Mejor esfuerzo: nunca lanza ni espera nada. */
export async function precalentarNube(): Promise<void> {
  try {
    await invoke("precalentar_nube");
  } catch {
    // Sin nube, sin sesión o sin conexión no hay nada que calentar, y no vale
    // la pena molestar a quien opera por eso.
  }
}

export function listarIngresosProveedorRemotos(): Promise<IngresoProveedorRemoto[]> {
  return invoke("listar_ingresos_proveedor_remotos");
}

/** Mismo motivo que `cerrarIngresoRemoto`, pero para el historial de
 * proveedores (`historial_ingresos_proveedor_sitio`). */
export async function cerrarIngresoProveedorRemoto(uuid: string): Promise<void> {
  await invoke("cerrar_ingreso_proveedor_remoto", { uuid });
  solicitarSincronizacionNube();
}

/** Ingreso por correo abierto por el otro dispositivo del sitio (caché
 * `ingresos_correo_remotos`, `comandos::nube::IngresoCorreoRemoto`). */
export interface IngresoCorreoRemoto {
  uuid: string;
  cedula: string;
  nombre: string;
  motivo: string;
  placa: string | null;
  gafete_numero: number;
  /** ISO 8601 (UTC). */
  hora_entrada: string;
  usuario_entrada_nombre: string;
}

export function listarIngresosCorreoRemotos(): Promise<IngresoCorreoRemoto[]> {
  return invoke("listar_ingresos_correo_remotos");
}

/** Mismo criterio que `cerrarIngresoProveedorRemoto`. */
export async function cerrarIngresoCorreoRemoto(uuid: string): Promise<void> {
  await invoke("cerrar_ingreso_correo_remoto", { uuid });
  solicitarSincronizacionNube();
}

/** Aviso cuando algo registrado en esta PC no llegó a la nube: lo decide el
 * núcleo (`nube::aviso_bandeja`). `critico`: ya no se reintenta solo.
 * `null` si no hay nada pendiente. */
export interface AvisoBandejaSalida {
  texto: string;
  critico: boolean;
}

export function avisoBandejaSalida(): Promise<AvisoBandejaSalida | null> {
  return invoke("aviso_bandeja_salida");
}

/** Último desfase medido entre el reloj de esta PC y el del servidor (ms,
 * positivo si la PC va adelantada), `null` si nunca se midió. Para la
 * telemetría de diagnóstico (latencia de los avisos en vivo). */
export function desfaseRelojMs(): Promise<number | null> {
  return invoke<number | null>("desfase_reloj_ms");
}

/** Unidad y etiqueta con que el panel registró esta PC (ver
 * `comandos::nube::identidad_equipo`). `null` mientras no hayan llegado de
 * la nube. */
export interface IdentidadEquipo {
  unidad: string | null;
  etiqueta: string | null;
}

export function obtenerIdentidadEquipo(): Promise<IdentidadEquipo> {
  return invoke<IdentidadEquipo>("identidad_equipo");
}
