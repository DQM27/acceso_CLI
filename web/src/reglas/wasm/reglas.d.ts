/* tslint:disable */
/* eslint-disable */

/**
 * ¿Este tipo admite la casilla "personal de ruta"?
 */
export function admitePersonalRuta(tipo_ingreso: string): boolean;

/**
 * Huella de las fuentes con las que se compiló este paquete (ver
 * `build.rs`). Los tests del panel la comparan con las fuentes del repo.
 */
export function huellaFuentes(): string;

/**
 * Todo nombre de persona o empresa, como se guarda: espacios de más fuera
 * y en mayúscula (`nombre::nombre_en_mayusculas`).
 */
export function nombreEnMayusculas(texto: string): string;

/**
 * Mientras se escribe en un campo de nombre: sólo mayúscula, sin tocar los
 * espacios (`nombre::nombre_mientras_se_escribe`).
 */
export function nombreMientrasSeEscribe(texto: string): string;

/**
 * Cédula en su forma única (sin guiones, espacios ni el cero del TSE), o
 * `undefined` si no es una cédula de contratista (9 a 13 dígitos).
 */
export function normalizarCedulaContratista(texto: string): string | undefined;

/**
 * Documento de un visitante en su forma única (la misma que reconoce el
 * check-in de la portería), o `undefined` si no es válido. Admite
 * pasaportes con letras.
 */
export function normalizarDocumentoVisitante(texto: string): string | undefined;

/**
 * ¿Este tipo exige gafete al entrar?
 */
export function requiereGafete(tipo_ingreso: string, personal_ruta: boolean): boolean;

/**
 * ¿Este tipo (o ser personal de ruta) exige fecha de PRAIND?
 */
export function requierePraind(tipo_ingreso: string, personal_ruta: boolean): boolean;

/**
 * Los tipos que se pueden elegir para un contratista nuevo, en el orden de
 * siempre.
 */
export function tiposIngresoSeleccionables(): string[];

/**
 * Valida una cita nueva con las reglas del núcleo
 * (`control_acceso_reglas::cita::validar_cita`).
 *
 * `datos`: `{ fecha_desde, fecha_hasta, hora_estimada?, motivo?, sitios,
 * visitantes: [{ nombre, cedula, empresa?, placa_vehiculo? }] }`, tal cual
 * del formulario. `hoy`: la fecha de Costa Rica, `"AAAA-MM-DD"`.
 *
 * Devuelve `{ ok: true, cita }` con los datos normalizados (cédula en su
 * forma única, textos recortados, vacíos como `null`), o `{ ok: false,
 * errores: [{ campo, mensaje }] }` con todos los problemas.
 */
export function validarCita(datos: any, hoy: string): any;

/**
 * Valida un contratista con todas las reglas de criterio
 * (`control_acceso_reglas::contratista::validar_contratista`).
 *
 * `datos`: `{ cedula, nombre, tipo_ingreso, fecha_vencimiento_praind,
 * es_personal_ruta?, tiene_acceso? }`. `hoy`: la fecha de Costa Rica,
 * `"AAAA-MM-DD"`. `anterior` (sólo al editar, si no `undefined`/`null`): lo
 * que tenía guardado, `{ tipo_ingreso, es_personal_ruta?,
 * fecha_vencimiento_praind }`; con él, a alguien con el PRAIND ya vencido se
 * le puede corregir el nombre o quitar el acceso sin cambiar la fecha.
 *
 * Devuelve `{ ok: true, contratista }` con los datos normalizados, o
 * `{ ok: false, codigo, mensaje }` con el primer motivo que falla.
 */
export function validarContratista(datos: any, hoy: string, anterior: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly admitePersonalRuta: (a: number, b: number) => number;
    readonly huellaFuentes: () => [number, number];
    readonly nombreEnMayusculas: (a: number, b: number) => [number, number];
    readonly nombreMientrasSeEscribe: (a: number, b: number) => [number, number];
    readonly normalizarCedulaContratista: (a: number, b: number) => [number, number];
    readonly normalizarDocumentoVisitante: (a: number, b: number) => [number, number];
    readonly requiereGafete: (a: number, b: number, c: number) => number;
    readonly requierePraind: (a: number, b: number, c: number) => number;
    readonly tiposIngresoSeleccionables: () => [number, number];
    readonly validarCita: (a: any, b: number, c: number) => [number, number, number];
    readonly validarContratista: (a: any, b: number, c: number, d: any) => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
