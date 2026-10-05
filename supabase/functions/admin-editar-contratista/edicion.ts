// Edición de un contratista desde el panel, sin red ni base de datos: valida
// con las reglas del núcleo (`_shared/reglas.ts`), pasando lo que tenía
// guardado, y usa los `puertos` para lo que necesita datos. Separada de
// `index.ts` para probarla con `deno test` (edicion.test.ts).

import {
  type EmpresaEncontrada,
  MENSAJE_CEDULA_CON_INGRESO_ACTIVO,
  MENSAJE_CEDULA_DUPLICADA,
  MENSAJE_CONTRATISTA_NO_ENCONTRADO,
  MENSAJE_EMPRESA_NO_ENCONTRADA,
  type ResultadoGuardar,
  texto,
  UUID,
} from "../_shared/contratistas.ts";
import { type ContratistaValido, normalizarCedulaContratista, validarContratista } from "../_shared/reglas.ts";

/** Lo que el contratista tiene guardado hoy. */
export interface ContratistaGuardado {
  id: string;
  identificacion: string;
  tipo_ingreso: string;
  es_personal_ruta: boolean;
  fecha_vencimiento_praind: string | null;
}

export interface Puertos {
  /** El correo de quien llama si es administrador del panel, o null. */
  autorizar(): Promise<string | null>;
  buscarContratista(id: string): Promise<ContratistaGuardado | null>;
  /** La empresa con ese id, o null. */
  buscarEmpresa(id: string): Promise<EmpresaEncontrada | null>;
  /** ¿Tiene un ingreso abierto en alguna unidad? (por id o por cédula). */
  estaAdentro(contratista: ContratistaGuardado): Promise<boolean>;
  /** Guarda y devuelve la fila; `"cedula_duplicada"` si el índice único la rechazó. */
  actualizar(id: string, contratista: ContratistaValido, empresa: EmpresaEncontrada): Promise<ResultadoGuardar>;
}

export type RespuestaEdicion =
  | { estado: 200; cuerpo: Record<string, unknown>; correo: string }
  | { estado: 400 | 401 | 404 | 409 | 422 | 500; cuerpo: { error: string; detail: string } };

export async function procesarEdicion(cuerpo: unknown, hoy: string, puertos: Puertos): Promise<RespuestaEdicion> {
  const datos = typeof cuerpo === "object" && cuerpo !== null ? cuerpo as Record<string, unknown> : {};
  const id = texto(datos.id);
  const empresaId = texto(datos.empresa_id);

  // Las tres consultas son independientes: van a la vez. Si quien llama no
  // es administrador, lo encontrado se descarta sin usarlo.
  const [correo, actual, empresa] = await Promise.all([
    puertos.autorizar(),
    UUID.test(id) ? puertos.buscarContratista(id) : Promise.resolve(null),
    UUID.test(empresaId) ? puertos.buscarEmpresa(empresaId) : Promise.resolve(null),
  ]);

  // 1. Sólo un administrador del panel.
  if (!correo) {
    return { estado: 401, cuerpo: { error: "unauthorized", detail: "No tiene permiso para editar contratistas." } };
  }
  if (typeof cuerpo !== "object" || cuerpo === null) {
    return { estado: 400, cuerpo: { error: "bad_request", detail: "Faltan datos del contratista" } };
  }
  if (!actual) {
    return {
      estado: 404,
      cuerpo: { error: "contratista_no_encontrado", detail: MENSAJE_CONTRATISTA_NO_ENCONTRADO },
    };
  }

  // 2. Reglas de criterio del núcleo, con lo que tenía guardado: a alguien
  //    con el PRAIND ya vencido se le corrige el nombre o se le quita el
  //    acceso sin tener que cambiar la fecha, y a uno viejo "por correo" se
  //    le corrigen los datos sin cambiarle el tipo.
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
    {
      tipo_ingreso: actual.tipo_ingreso,
      es_personal_ruta: actual.es_personal_ruta,
      fecha_vencimiento_praind: actual.fecha_vencimiento_praind,
    },
  );
  if (!resultado.ok) {
    return { estado: 422, cuerpo: { error: resultado.codigo, detail: resultado.mensaje } };
  }

  // 3. La empresa existe.
  if (!empresa) {
    return { estado: 404, cuerpo: { error: "empresa_no_encontrada", detail: MENSAJE_EMPRESA_NO_ENCONTRADA } };
  }

  // 4. No se cambia la cédula de quien está adentro (misma regla que el
  //    núcleo, `CedulaConIngresoActivo`): su ingreso abierto quedaría con la
  //    vieja y, con la nueva, podría volver a entrar sin que nada lo frene.
  //    La misma cédula escrita distinto no es un cambio.
  const cedulaGuardada = normalizarCedulaContratista(actual.identificacion) ?? actual.identificacion;
  if (resultado.contratista.cedula !== cedulaGuardada && await puertos.estaAdentro(actual)) {
    return {
      estado: 409,
      cuerpo: { error: "cedula_con_ingreso_activo", detail: MENSAJE_CEDULA_CON_INGRESO_ACTIVO },
    };
  }

  // 5. Guardar. La cédula repetida la frena el índice único
  //    `contratistas_cedula_normalizada_key`.
  const guardado = await puertos.actualizar(actual.id, resultado.contratista, empresa);
  if (!guardado.ok) {
    return guardado.motivo === "cedula_duplicada"
      ? { estado: 409, cuerpo: { error: "cedula_duplicada", detail: MENSAJE_CEDULA_DUPLICADA } }
      : { estado: 500, cuerpo: { error: "error_al_guardar", detail: "No se pudo guardar el contratista" } };
  }
  return { estado: 200, cuerpo: guardado.fila, correo };
}
