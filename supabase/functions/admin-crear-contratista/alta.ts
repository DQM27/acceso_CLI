// Alta de un contratista desde el panel, sin red ni base de datos: valida con
// las reglas del núcleo (`_shared/reglas.ts`) y usa los `puertos` para lo que
// necesita datos (la empresa existe, guardar). Separada de `index.ts` para
// probarla con `deno test` (alta.test.ts).

import { type ContratistaValido, validarContratista } from "../_shared/reglas.ts";

export interface EmpresaEncontrada {
  id: string;
  nombre: string;
}

export interface Puertos {
  /** La empresa con ese id, o null (igual que la función SQL que reemplaza: no exige que esté activa). */
  buscarEmpresa(id: string): Promise<EmpresaEncontrada | null>;
  /** Guarda y devuelve la fila creada; `"cedula_duplicada"` si el índice
   * único de cédula la rechazó. */
  insertar(contratista: ContratistaValido, empresa: EmpresaEncontrada): Promise<
    { ok: true; fila: Record<string, unknown> } | { ok: false; motivo: "cedula_duplicada" | "error"; detalle?: string }
  >;
}

export type RespuestaAlta =
  | { estado: 200; cuerpo: Record<string, unknown> }
  | { estado: 400 | 404 | 409 | 422 | 500; cuerpo: { error: string; detail: string } };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Mismos textos que las apps (`mensajes::mensaje_contratista` del núcleo)
 * para lo que no es una regla de criterio. */
const MENSAJE_EMPRESA_NO_ENCONTRADA = "La empresa seleccionada ya no existe";
const MENSAJE_CEDULA_DUPLICADA = "Ya existe un contratista con esa cédula";

export async function procesarAlta(cuerpo: unknown, hoy: string, puertos: Puertos): Promise<RespuestaAlta> {
  if (typeof cuerpo !== "object" || cuerpo === null) {
    return { estado: 400, cuerpo: { error: "bad_request", detail: "Faltan datos del contratista" } };
  }
  const datos = cuerpo as Record<string, unknown>;
  const texto = (valor: unknown) => (typeof valor === "string" ? valor : "");

  // 1. Reglas de criterio: las del núcleo, con su mensaje.
  const resultado = validarContratista(
    {
      cedula: texto(datos.cedula),
      nombre: texto(datos.nombre),
      tipo_ingreso: texto(datos.tipo_ingreso),
      fecha_vencimiento_praind: typeof datos.fecha_vencimiento_praind === "string"
        ? datos.fecha_vencimiento_praind
        : null,
      // El panel no tiene la casilla "personal de ruta": siempre false.
      es_personal_ruta: false,
      tiene_acceso: datos.con_acceso !== false,
    },
    hoy,
  );
  if (!resultado.ok) {
    return { estado: 422, cuerpo: { error: resultado.codigo, detail: resultado.mensaje } };
  }

  // 2. La empresa existe (dato, no regla de criterio).
  const empresaId = texto(datos.empresa_id);
  const empresa = UUID.test(empresaId) ? await puertos.buscarEmpresa(empresaId) : null;
  if (!empresa) {
    return { estado: 404, cuerpo: { error: "empresa_no_encontrada", detail: MENSAJE_EMPRESA_NO_ENCONTRADA } };
  }

  // 3. Guardar. La cédula repetida la decide el índice único
  //    `contratistas_cedula_normalizada_key` (también frena dos altas a la vez).
  const guardado = await puertos.insertar(resultado.contratista, empresa);
  if (!guardado.ok) {
    return guardado.motivo === "cedula_duplicada"
      ? { estado: 409, cuerpo: { error: "cedula_duplicada", detail: MENSAJE_CEDULA_DUPLICADA } }
      : { estado: 500, cuerpo: { error: "error_al_guardar", detail: "No se pudo guardar el contratista" } };
  }
  return { estado: 200, cuerpo: guardado.fila };
}
