// Cálculos puros de la telemetría de diagnóstico (ver `telemetria.ts`):
// agregan mediciones y las resumen con los MISMOS nombres de campo que la
// app Android (`TelemetriaCalculos.kt`), así una misma consulta sirve para
// las dos plataformas. Sin Tauri ni DOM, para poder probarlos.

type Fila = Record<string, unknown>;

/** Versión del formato de las filas de telemetría (`datos.esquema`); la misma
 * que `ESQUEMA_TELEMETRIA` del teléfono. Ver `docs/telemetria-contrato.md`. */
export const ESQUEMA_TELEMETRIA = 2;

/** Nombre canónico de una operación, igual en escritorio y móvil
 * (`nombreCanonicoOperacion` en `TelemetriaCalculos.kt`, mismos ejemplos en los
 * tests de las dos): snake_case y sin los sufijos de versiones viejas
 * (`_con_secreto`, `_verificado`), para poder comparar entre versiones y entre
 * plataformas. Idempotente. */
export function nombreCanonicoOperacion(nombre: string): string {
  const sufijosViejos = ["_con_secreto", "_verificado"];
  let canonico = nombre
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2")
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .toLowerCase();
  let quitoAlgo = true;
  while (quitoAlgo) {
    quitoAlgo = false;
    for (const sufijo of sufijosViejos) {
      if (canonico.length > sufijo.length && canonico.endsWith(sufijo)) {
        canonico = canonico.slice(0, canonico.length - sufijo.length);
        quitoAlgo = true;
      }
    }
  }
  return canonico;
}

/** Tipo de red en palabras fijas (lista cerrada). En escritorio el navegador
 * sólo dice si hay red; no distingue wifi de cable: `con_red`. */
export function tipoDeRed(enLinea: boolean): "sin_red" | "con_red" {
  return enLinea ? "con_red" : "sin_red";
}

/** Percentil por rango más cercano sobre valores ya ordenados. */
export function percentil(ordenados: readonly number[], p: number): number | null {
  if (ordenados.length === 0) return null;
  const indice = Math.min(Math.max(Math.ceil(p * ordenados.length) - 1, 0), ordenados.length - 1);
  return ordenados[indice];
}

function redondeado(valor: number | null | undefined, decimales = 1): number | null {
  if (valor === null || valor === undefined) return null;
  const factor = 10 ** decimales;
  return Math.round(valor * factor) / factor;
}

function sumar(conteos: Map<string, number>, clave: string) {
  conteos.set(clave, (conteos.get(clave) ?? 0) + 1);
}

function ordenado(conteos: Map<string, number>): Record<string, number> {
  return Object.fromEntries([...conteos.entries()].sort(([a], [b]) => a.localeCompare(b)));
}

/** Tipos de error de un comando: lista FIJA, la misma que usa el teléfono
 * (`docs/telemetria-contrato.md`). Es lo único que sale de la app de un error:
 * nunca su mensaje, que puede traer nombres, cédulas o gafetes. */
export const TIPOS_ERROR = [
  "sin_red",
  "no_autenticado",
  "credenciales",
  "sin_autorizacion",
  "reloj",
  "rechazado",
  "conflicto",
  "no_encontrado",
  "validacion",
  "base_de_datos",
  "interno",
  "cancelada",
  "otro",
] as const;
export type TipoError = (typeof TIPOS_ERROR)[number];

// Un comando de Tauri falla con el texto del error del núcleo (en español, ver
// `src/services/error.rs`). Se lee sólo aquí, para elegir un tipo; el texto no
// se guarda ni se envía. El orden importa: gana la primera coincidencia. Un
// mensaje que no coincide con nada queda como "otro": si crece, hay un mensaje
// nuevo que enseñarle a esta lista.
const PATRONES_ERROR: ReadonlyArray<readonly [TipoError, RegExp]> = [
  [
    "sin_red",
    /sin conexi[oó]n|no se pudo conectar|error sending request|timed out|tiempo de espera|dns error|connection (refused|reset|closed)|network|offline|sin red/i,
  ],
  ["no_autenticado", /no hay una sesi[oó]n activa|sesi[oó]n .*(expir|no existe|inactiva)|no autenticad/i],
  ["sin_autorizacion", /no est[aá] autorizada/i],
  ["reloj", /reloj del equipo/i],
  ["credenciales", /credenciales|contrase[ñn]a (actual )?(es )?incorrecta|usuario inactivo|no tiene contrase[ñn]a/i],
  ["rechazado", /acceso denegado|praind (est[aá] )?vencid|no hay ninguna visita|no admite|ya no se puede elegir/i],
  [
    "conflicto",
    /ya tiene (un |una )|ya est[aá] adentro|ya est[aá] asignad|no est[aá] disponible|ya existe|movimiento activo/i,
  ],
  ["no_encontrado", /no encontrad[oa]|no est[aá] registrad|no est[aá] asignado actualmente|no est[aá] activo/i],
  [
    "validacion",
    /es obligatori|inv[aá]lid|no es v[aá]lid|debe tener|no puede tener|s[oó]lo n[uú]meros|no se puede indicar|no puede indicar|falta la placa|anterior (al|a la) (ingreso|entrada)/i,
  ],
  ["base_de_datos", /base de datos|sqlite|database|disk i\/o|database is locked/i],
  ["interno", /error interno|panic/i],
];

/** El tipo de un comando que falló. `enLinea` es `navigator.onLine`: si el
 * mensaje no dice nada y no hay red, lo más probable es que fue la red. */
export function clasificarErrorComando(error: unknown, enLinea = true): TipoError {
  if (typeof DOMException !== "undefined" && error instanceof DOMException && error.name === "AbortError") {
    return "cancelada";
  }
  const texto = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  for (const [tipo, patron] of PATRONES_ERROR) {
    if (patron.test(texto)) return tipo;
  }
  return enLinea ? "otro" : "sin_red";
}

/** Tiempos de cada comando de Tauri (IPC incluido), por nombre. Mismo
 * formato que `llamada_nucleo` del teléfono. */
type AcumuladoLlamadas = { ms: number[]; errores: number; porTipo: Map<string, number> };

export class AgregadorLlamadas {
  private readonly porNombre = new Map<string, AcumuladoLlamadas>();

  /** `tipoError` sólo importa si `ok` es falso; sin él, el error cuenta como
   * "otro". */
  registrar(nombre: string, ms: number, ok: boolean, tipoError?: TipoError) {
    const entrada: AcumuladoLlamadas = this.porNombre.get(nombre) ?? { ms: [], errores: 0, porTipo: new Map() };
    entrada.ms.push(ms);
    if (!ok) {
      entrada.errores += 1;
      sumar(entrada.porTipo, tipoError ?? "otro");
    }
    this.porNombre.set(nombre, entrada);
  }

  vaciar(): Fila[] {
    const filas = [...this.porNombre.entries()].map(([nombre, { ms, errores, porTipo }]) => {
      const ordenados = [...ms].sort((a, b) => a - b);
      return {
        nombre,
        llamadas: ms.length,
        errores,
        // Sólo si hubo errores: la fila normal no cambia.
        ...(errores > 0 ? { errores_por_tipo: ordenado(porTipo) } : {}),
        p50_ms: redondeado(percentil(ordenados, 0.5)),
        p90_ms: redondeado(percentil(ordenados, 0.9)),
        max_ms: redondeado(ordenados.at(-1)),
        total_ms: redondeado(ms.reduce((a, b) => a + b, 0)),
      };
    });
    this.porNombre.clear();
    return filas;
  }
}

/** Canal de avisos en vivo: conexión, avisos recibidos y su latencia. Mismo
 * formato que `realtime` del teléfono (`AgregadorRealtime` en Kotlin). */
export class AgregadorRealtime {
  private intentosConexion = 0;
  private msHastaSuscribir: number[] = [];
  private caidas = new Map<string, number>();
  private errores = new Map<string, number>();
  private msConectado: number[] = [];
  private avisosPorTabla = new Map<string, number>();
  private ecosPropios = 0;
  private bytes = 0;
  private aplicados = 0;
  private noAplicados = 0;
  private msAplicar: number[] = [];
  private latenciasMs: number[] = [];
  private desfaseRelojMs: number | null = null;
  private vacio = true;

  /** Desfase del reloj con que se corrigen las latencias (`null` = no se
   * midió y la latencia incluye el desfase). */
  desfaseReloj(ms: number | null) {
    this.desfaseRelojMs = ms;
  }

  conectando() {
    this.intentosConexion += 1;
    this.vacio = false;
  }

  suscrito(msDesdeIntento: number) {
    this.msHastaSuscribir.push(msDesdeIntento);
    this.vacio = false;
  }

  /** El canal dejó de estar suscrito (`motivo`: estado del canal o
   * "renovacion" del token) tras `msConectadoAhora` de conexión. */
  terminado(motivo: string, msConectadoAhora: number | null) {
    sumar(this.caidas, motivo);
    if (msConectadoAhora !== null) this.msConectado.push(msConectadoAhora);
    this.vacio = false;
  }

  error(tipo: string) {
    sumar(this.errores, tipo);
    this.vacio = false;
  }

  /** Un aviso recibido. `latenciaMs`: hora de esta PC (corregida con el
   * desfase medido) menos la del servidor al escribir (`changed_at`). */
  aviso(tabla: string | undefined, bytesAviso: number, ecoPropio: boolean, latenciaMs: number | null) {
    this.vacio = false;
    this.bytes += bytesAviso;
    if (latenciaMs !== null) this.latenciasMs.push(latenciaMs);
    if (ecoPropio) {
      this.ecosPropios += 1;
      return;
    }
    sumar(this.avisosPorTabla, tabla ?? "(sin tabla)");
  }

  /** Resultado de guardar en la base local la fila que trajo un aviso. */
  aplicado(ok: boolean, ms: number) {
    if (ok) this.aplicados += 1;
    else this.noAplicados += 1;
    this.msAplicar.push(ms);
    this.vacio = false;
  }

  /** Resumen y vacía lo acumulado; `null` si no pasó nada. */
  vaciar(): Fila | null {
    if (this.vacio) return null;
    const p = (valores: number[], q: number) => redondeado(percentil([...valores].sort((a, b) => a - b), q));
    const max = (valores: number[]) => (valores.length ? redondeado(Math.max(...valores)) : null);
    const min = (valores: number[]) => (valores.length ? redondeado(Math.min(...valores)) : null);
    const resumen: Fila = {
      intentos_conexion: this.intentosConexion,
      ms_hasta_suscribir_p50: p(this.msHastaSuscribir, 0.5),
      ms_hasta_suscribir_max: max(this.msHastaSuscribir),
      fin_de_conexion: ordenado(this.caidas),
      errores: ordenado(this.errores),
      minutos_conectado_max: this.msConectado.length
        ? redondeado(Math.max(...this.msConectado) / 60_000)
        : null,
      avisos_por_tabla: ordenado(this.avisosPorTabla),
      avisos: [...this.avisosPorTabla.values()].reduce((a, b) => a + b, 0),
      ecos_propios: this.ecosPropios,
      kb_recibidos: redondeado(this.bytes / 1024),
      aplicados: this.aplicados,
      no_aplicados: this.noAplicados,
      ms_aplicar_p50: p(this.msAplicar, 0.5),
      ms_aplicar_max: max(this.msAplicar),
      latencia_ms_p50: p(this.latenciasMs, 0.5),
      latencia_ms_p90: p(this.latenciasMs, 0.9),
      latencia_ms_min: min(this.latenciasMs),
      latencia_ms_max: max(this.latenciasMs),
      latencia_corregida: this.desfaseRelojMs !== null,
      desfase_reloj_ms: this.desfaseRelojMs,
    };
    this.intentosConexion = 0;
    this.msHastaSuscribir = [];
    this.caidas.clear();
    this.errores.clear();
    this.msConectado = [];
    this.avisosPorTabla.clear();
    this.ecosPropios = 0;
    this.bytes = 0;
    this.aplicados = 0;
    this.noAplicados = 0;
    this.msAplicar = [];
    this.latenciasMs = [];
    this.vacio = true;
    return resumen;
  }
}

/** Tareas largas del hilo de la interfaz (> 50 ms: la ventana no responde
 * mientras duran). Equivale a `frames` del teléfono. */
export class AgregadorBloqueos {
  private duraciones: number[] = [];

  registrar(ms: number) {
    this.duraciones.push(ms);
  }

  vaciar(): Fila | null {
    if (this.duraciones.length === 0) return null;
    const ordenados = [...this.duraciones].sort((a, b) => a - b);
    const resumen = {
      tareas_largas: ordenados.length,
      ms_bloqueado: redondeado(ordenados.reduce((a, b) => a + b, 0)),
      ms_p50: redondeado(percentil(ordenados, 0.5)),
      ms_max: redondeado(ordenados.at(-1)),
      mayores_a_200ms: ordenados.filter((ms) => ms > 200).length,
    };
    this.duraciones = [];
    return resumen;
  }
}

/** Milisegundos desde `instanteIso` (hora del servidor) hasta `ahoraMs`
 * (hora de esta PC) corregida por `desfaseMs` (PC menos servidor). */
export function latenciaDesde(instanteIso: string, ahoraMs: number, desfaseMs: number | null): number | null {
  const instante = Date.parse(instanteIso);
  if (Number.isNaN(instante)) return null;
  return Math.round(ahoraMs - (desfaseMs ?? 0) - instante);
}

/** Sólo el tipo de un error (`TypeError`, `RangeError`...), nunca el
 * mensaje: puede traer datos de la pantalla. */
export function tipoDeError(error: unknown): string {
  if (error instanceof Error) return error.name || "Error";
  if (typeof error === "string") return "string";
  return typeof error;
}
