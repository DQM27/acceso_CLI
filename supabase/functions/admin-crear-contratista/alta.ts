// Alta de un contratista desde el panel, sin red ni base de datos: valida con
// las reglas del núcleo (`_shared/reglas.ts`) y usa los `puertos` para lo que
// necesita datos (quién llama, la empresa existe, guardar). Separada de
// `index.ts` para probarla con `deno test` (alta.test.ts).

import {
  type EmpresaEncontrada,
  MENSAJE_CEDULA_DUPLICADA,
  MENSAJE_EMPRESA_NO_ENCONTRADA,
  type ResultadoGuardar,
  texto,
  UUID,
} from "../_shared/contratistas.ts";
import { type ContratistaValido, validarContratista } from "../_shared/reglas.ts";

export type { EmpresaEncontrada } from "../_shared/contratistas.ts";

export interface Puertos {
  /** El correo de quien llama si es administrador del panel, o null. */
  autorizar(): Promise<string | null>;
  /** La empresa con ese id, o null (igual que la función SQL que reemplaza: no exige que esté activa). */
  buscarEmpresa(id: string): Promise<EmpresaEncontrada | null>;
  /** Guarda y devuelve la fila creada; `"cedula_duplicada"` si el índice
   * único de cédula la rechazó. */
  insertar(contratista: ContratistaValido, empresa: EmpresaEncontrada): Promise<ResultadoGuardar>;
}

export type RespuestaAlta =
  | { estado: 200; cuerpo: Record<string, unknown>; correo: string }
  | { estado: 400 | 401 | 404 | 409 | 422 | 500; cuerpo: { error: string; detail: string } };

export async function procesarAlta(cuerpo: unknown, hoy: string, puertos: Puertos): Promise<RespuestaAlta> {
  const datos = typeof cuerpo === "object" && cuerpo !== null ? cuerpo as Record<string, unknown> : {};

  // Quién llama y la empresa se consultan A LA VEZ: son independientes, y
  // así el alta hace un viaje a la base menos (la autorización ya son dos:
  // la sesión y `administradores_panel`). Si quien llama no es
  // administrador, la empresa encontrada se descarta sin usarla.
  const empresaId = texto(datos.empresa_id);
  const [correo, empresa] = await Promise.all([
    puertos.autorizar(),
    UUID.test(empresaId) ? puertos.buscarEmpresa(empresaId) : Promise.resolve(null),
  ]);

  // 1. Sólo un administrador del panel.
  if (!correo) {
    return { estado: 401, cuerpo: { error: "unauthorized", detail: "No tiene permiso para crear contratistas." } };
  }
  if (typeof cuerpo !== "object" || cuerpo === null) {
    return { estado: 400, cuerpo: { error: "bad_request", detail: "Faltan datos del contratista" } };
  }

  // 2. Reglas de criterio: las del núcleo, con su mensaje.
  const resultado = validarContratista(
    {
      cedula: texto(datos.cedula),
      nombre: texto(datos.nombre),
      tipo_ingreso: texto(datos.tipo_ingreso),
      fecha_vencimiento_praind: typeof datos.fecha_vencimiento_praind === "string"
        ? datos.fecha_vencimiento_praind
        : null,
      es_personal_ruta: datos.es_personal_ruta === true,
      tiene_acceso: datos.con_acceso !== false,
    },
    hoy,
  );
  if (!resultado.ok) {
    return { estado: 422, cuerpo: { error: resultado.codigo, detail: resultado.mensaje } };
  }

  // 3. La empresa existe (dato, no regla de criterio).
  if (!empresa) {
    return { estado: 404, cuerpo: { error: "empresa_no_encontrada", detail: MENSAJE_EMPRESA_NO_ENCONTRADA } };
  }

  // 4. Guardar. La cédula repetida la decide el índice único
  //    `contratistas_cedula_normalizada_key` (también frena dos altas a la vez).
  const guardado = await puertos.insertar(resultado.contratista, empresa);
  if (!guardado.ok) {
    return guardado.motivo === "cedula_duplicada"
      ? { estado: 409, cuerpo: { error: "cedula_duplicada", detail: MENSAJE_CEDULA_DUPLICADA } }
      : { estado: 500, cuerpo: { error: "error_al_guardar", detail: "No se pudo guardar el contratista" } };
  }
  return { estado: 200, cuerpo: guardado.fila, correo };
}
