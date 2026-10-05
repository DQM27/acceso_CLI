import type { TipoIngreso } from "../api/contratistas";
import { reglas, type EstadoAnteriorContratista, type ResultadoValidacion } from "../reglas";
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
 * esta pantalla lo filtre. Al editar, `actual` (el tipo que ya tiene) se
 * suma aunque esté retirado: se puede dejar como está, no elegir de nuevo. */
export function tiposIngreso(actual?: TipoIngreso): { valor: TipoIngreso; etiqueta: string }[] {
  const elegibles = reglas.tiposIngresoSeleccionables();
  const valores = actual && !elegibles.includes(actual) ? [...elegibles, actual] : elegibles;
  return valores.map((valor) => ({
    valor,
    etiqueta: elegibles.includes(valor) ? ETIQUETA_TIPO[valor] : `${ETIQUETA_TIPO[valor]} (retirado)`,
  }));
}

/** Si el tipo admite la casilla "personal de ruta" (núcleo). */
export function admitePersonalRuta(tipo: TipoIngreso): boolean {
  return reglas.admitePersonalRuta(tipo);
}

/** Si el formulario muestra la fecha de vencimiento del PRAIND: lo decide el
 * núcleo (`requiere_praind_de`, también por ser personal de ruta). A quien no
 * va a tener acceso no se le pide (misma regla que `validar_contratista`). */
export function pidePraind(tipo: TipoIngreso, conAcceso: boolean, personalRuta = false): boolean {
  return conAcceso && reglas.requierePraind(tipo, personalRuta);
}

/** Revisa los datos ANTES de enviarlos, con las mismas reglas que aplica el
 * servidor al guardar (`validar_contratista` del núcleo, vía WebAssembly), y
 * devuelve el mensaje del primer problema o `null`. Al editar, `anterior` es lo
 * que tenía guardado (con eso, a alguien con el PRAIND ya vencido se le corrige
 * el nombre sin tocar la fecha). Es sólo para avisar al toque: la decisión la
 * toman las Edge Functions `admin-crear-contratista` y
 * `admin-editar-contratista`, que validan otra vez con el mismo código. La
 * empresa (un dato que no es de las reglas) se pide acá. */
export function errorAntesDeEnviar(
  valores: {
    empresaId: string;
    cedula: string;
    nombre: string;
    tipo: TipoIngreso;
    praind: string | null;
    conAcceso: boolean;
    personalRuta?: boolean;
    anterior?: EstadoAnteriorContratista;
  },
  hoy: string = fechaYMD(new Date()),
): string | null {
  const resultado: ResultadoValidacion = reglas.validarContratista(
    {
      cedula: valores.cedula,
      nombre: valores.nombre,
      tipo_ingreso: valores.tipo,
      fecha_vencimiento_praind: valores.praind,
      es_personal_ruta: valores.personalRuta ?? false,
      tiene_acceso: valores.conAcceso,
    },
    hoy,
    valores.anterior,
  );
  if (!resultado.ok) return resultado.mensaje;
  return valores.empresaId === "" ? "Elija la empresa" : null;
}
