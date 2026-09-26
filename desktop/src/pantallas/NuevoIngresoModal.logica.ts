import type { ContratistaResumen } from "../api";
import { fechaYMD } from "../tiempo";

/** Extraída de `confirmarIngreso` para poder testearla sin renderizar el
 * modal ni mockear la API -- mismo criterio que `esquema` en los
 * Formulario*. `requiereGafete` en `false` siempre es válido, con `null`
 * (nunca lee `texto` en ese caso: un texto tipeado y después descartado por
 * cambiar de contratista no debería poder colarse). */
export function validarGafete(
  texto: string,
  requiereGafete: boolean,
): { valido: true; numero: number | null } | { valido: false; mensaje: string } {
  if (!requiereGafete) return { valido: true, numero: null };
  const recortado = texto.trim();
  const numero = Number.parseInt(recortado, 10);
  if (!recortado) return { valido: false, mensaje: "El gafete es requerido" };
  if (Number.isNaN(numero)) return { valido: false, mensaje: "Ingrese un número de gafete válido" };
  return { valido: true, numero };
}

export interface AvisoContratista {
  texto: string;
  /** Variable CSS del color del chip (ej. "var(--error)"). */
  color: string;
}

/** Chips de la lista de resultados del buscador: lo que el operador tiene
 * que ver ANTES de elegir a alguien (pedido del usuario 2026-09-23, que
 * además sacó la empresa de esa lista para darles lugar -- sigue en la
 * ficha al elegir). Sólo informativos: si puede o no entrar lo decide el
 * núcleo al elegir (`prepararIngreso`), no estos chips. `hoy` en
 * "AAAA-MM-DD", inyectable para el test. */
export function avisosContratista(
  contratista: Pick<
    ContratistaResumen,
    "tiene_ingreso_activo" | "tiene_acceso" | "fecha_vencimiento_praind"
  >,
  hoy: string = fechaYMD(new Date()),
): AvisoContratista[] {
  const avisos: AvisoContratista[] = [];
  if (contratista.tiene_ingreso_activo) avisos.push({ texto: "Adentro", color: "var(--acento)" });
  if (!contratista.tiene_acceso) avisos.push({ texto: "Sin acceso", color: "var(--error)" });
  if (contratista.fecha_vencimiento_praind && contratista.fecha_vencimiento_praind < hoy) {
    avisos.push({ texto: "PRAIND vencido", color: "var(--error)" });
  }
  return avisos;
}

/** Mismo criterio que `validarGafete`, pero para la placa -- obligatoria
 * cuando el medio es `"Vehiculo"`, descartada (nunca se manda) cuando es
 * `"Caminando"` (ver `confirmarIngreso`, que ni siquiera llama a esta
 * función en ese caso). Sin formato particular impuesto: las placas de
 * Costa Rica varían bastante (motos, vehículos de otras provincias,
 * temporales), no vale la pena una expresión regular frágil. */
export function validarPlaca(
  texto: string,
): { valido: true; placa: string } | { valido: false; mensaje: string } {
  const recortada = texto.trim();
  if (!recortada) return { valido: false, mensaje: "La placa es requerida" };
  return { valido: true, placa: recortada };
}
