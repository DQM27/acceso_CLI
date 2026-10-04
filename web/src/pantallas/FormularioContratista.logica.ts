import type { TipoIngreso } from "../api/contratistas";
import { reglas, type ResultadoValidacion } from "../reglas";
import { fechaYMD } from "../tiempo";

/** Texto que se lee en pantalla para cada tipo ("IN HOUSE", no "IN_HOUSE"). */
const ETIQUETA_TIPO: Record<TipoIngreso, string> = {
  PRAIND: "PRAIND",
  IN_HOUSE: "IN HOUSE",
  POR_CORREO: "POR CORREO",
  SWAT: "SWAT",
};

/** Tipos de ingreso que se pueden elegir, en el orden del formulario. Los
 * decide el núcleo (`tipo_ingreso_seleccionable`, vía WebAssembly): "POR
 * CORREO" ya no aparece porque el núcleo lo retiró (2026-10-03), no porque
 * esta pantalla lo filtre. */
export function tiposIngreso(): { valor: TipoIngreso; etiqueta: string }[] {
  return reglas.tiposIngresoSeleccionables().map((valor) => ({ valor, etiqueta: ETIQUETA_TIPO[valor] }));
}

/** Si el formulario muestra la fecha de vencimiento del PRAIND: lo decide el
 * núcleo (`requiere_praind_de`). A quien se crea con el acceso denegado no se
 * le pide (misma regla que `validar_contratista`). */
export function pidePraind(tipo: TipoIngreso, conAcceso: boolean): boolean {
  return conAcceso && reglas.requierePraind(tipo);
}

/** Revisa los datos ANTES de enviarlos, con las mismas reglas que aplica el
 * servidor al guardar (`validar_contratista` del núcleo, vía WebAssembly), y
 * devuelve el mensaje del primer problema o `null`. Es sólo para avisar al
 * toque: la decisión la toma la Edge Function `admin-crear-contratista`,
 * que valida otra vez con el mismo código. La empresa (un dato que no es de
 * las reglas) se pide acá. */
export function errorAntesDeEnviar(
  valores: {
    empresaId: string;
    cedula: string;
    nombre: string;
    tipo: TipoIngreso;
    praind: string | null;
    conAcceso: boolean;
  },
  hoy: string = fechaYMD(new Date()),
): string | null {
  const resultado: ResultadoValidacion = reglas.validarContratista(
    {
      cedula: valores.cedula,
      nombre: valores.nombre,
      tipo_ingreso: valores.tipo,
      fecha_vencimiento_praind: valores.praind,
      tiene_acceso: valores.conAcceso,
    },
    hoy,
  );
  if (!resultado.ok) return resultado.mensaje;
  return valores.empresaId === "" ? "Elija la empresa" : null;
}
