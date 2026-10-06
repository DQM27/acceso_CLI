// Acciones del panel sobre cuentas de anfitriones, sin red ni base de datos:
// usa los `puertos` para todo lo que toca Supabase. Separada de `index.ts`
// para probarla con `deno test` (acciones.test.ts).

import { esCorreoDeOperador, normalizarCorreo } from "../_shared/anfitriones.ts";
import { nombreEnMayusculas } from "../_shared/reglas.ts";

export interface FilaAnfitrion {
  correo: string;
  nombre: string;
  activo: boolean;
  auth_user_id: string | null;
}

export interface Puertos {
  /** El correo de quien llama si es administrador del panel, o null. */
  autorizar(): Promise<string | null>;
  esAdministrador(correo: string): Promise<boolean>;
  buscar(correo: string): Promise<FilaAnfitrion | null>;
  /** `false` si ya existía un anfitrión con ese correo. */
  insertar(correo: string, nombre: string): Promise<boolean>;
  borrar(correo: string): Promise<void>;
  /** Crea la cuenta de Auth con una contraseña aleatoria, o devuelve la que
   * ya existe con ese correo (que también recibe una aleatoria). */
  asegurarCuenta(correo: string): Promise<{ id: string; creada: boolean }>;
  borrarCuenta(id: string): Promise<void>;
  enlazar(correo: string, id: string): Promise<void>;
  /** Invalida la contraseña actual poniendo una aleatoria. */
  contrasenaAleatoria(id: string): Promise<void>;
  cerrarSesiones(id: string): Promise<void>;
  bloquear(id: string, bloquear: boolean): Promise<void>;
  generarCodigo(): string;
  /** Guarda el hash del código; devuelve cuándo vence (ISO). */
  emitirCodigo(correo: string, codigo: string, admin: string, accion: "crear" | "restablecer"): Promise<string>;
  /** Cambia `activo` (y borra el código pendiente al deshabilitar);
   * devuelve la cuenta de Auth enlazada, si hay. */
  cambiarEstado(correo: string, activo: boolean, admin: string): Promise<string | null>;
}

type Error4xx = 400 | 401 | 404 | 409;
export type Respuesta =
  | { estado: 200; cuerpo: Record<string, unknown>; admin: string; accion: string }
  | { estado: Error4xx | 500; cuerpo: { error: string; detail: string } };

const ACCIONES = ["crear", "restablecer", "deshabilitar", "habilitar"] as const;
type Accion = typeof ACCIONES[number];

function error(estado: Error4xx | 500, codigo: string, detalle: string): Respuesta {
  return { estado, cuerpo: { error: codigo, detail: detalle } };
}

export async function procesar(cuerpo: unknown, puertos: Puertos): Promise<Respuesta> {
  const admin = await puertos.autorizar();
  if (!admin) return error(401, "unauthorized", "No tiene permiso para administrar anfitriones.");

  const datos = typeof cuerpo === "object" && cuerpo !== null ? cuerpo as Record<string, unknown> : {};
  const accion = datos.accion as Accion;
  if (!ACCIONES.includes(accion)) return error(400, "bad_request", "Acción desconocida.");
  const correo = normalizarCorreo(datos.correo);
  if (!correo) return error(400, "correo_invalido", "Escriba un correo válido.");

  if (accion === "crear") return await crear(correo, datos.nombre, admin, puertos);

  const fila = await puertos.buscar(correo);
  if (!fila) return error(404, "no_existe", "No hay un anfitrión con ese correo.");

  if (accion === "restablecer") {
    if (!fila.activo) return error(409, "deshabilitado", "La cuenta está deshabilitada. Habilítela primero.");
    if (await puertos.esAdministrador(correo)) return errorAdministrador();
    let id = fila.auth_user_id;
    if (!id) {
      id = (await puertos.asegurarCuenta(correo)).id;
      await puertos.enlazar(correo, id);
    } else {
      await puertos.contrasenaAleatoria(id);
    }
    await puertos.cerrarSesiones(id);
    const codigo = puertos.generarCodigo();
    const vence = await puertos.emitirCodigo(correo, codigo, admin, "restablecer");
    return { estado: 200, cuerpo: { correo, nombre: fila.nombre, codigo, vence }, admin, accion };
  }

  const activo = accion === "habilitar";
  const id = await puertos.cambiarEstado(correo, activo, admin);
  // Una cuenta que además es de administrador del panel no se bloquea en
  // Auth: le cortaría el panel. `activo = false` ya le quita las visitas.
  if (id && !(await puertos.esAdministrador(correo))) {
    await puertos.bloquear(id, !activo);
    if (!activo) await puertos.cerrarSesiones(id);
  }
  return { estado: 200, cuerpo: { correo, activo }, admin, accion };
}

function errorAdministrador(): Respuesta {
  return error(
    409,
    "es_administrador",
    "Ese correo es de un administrador del panel. Para anfitriones use otro correo: una contraseña no debe dar acceso al panel.",
  );
}

async function crear(correo: string, nombreCrudo: unknown, admin: string, puertos: Puertos): Promise<Respuesta> {
  const nombre = typeof nombreCrudo === "string" ? nombreEnMayusculas(nombreCrudo) : "";
  if (!nombre || nombre.length > 120) return error(400, "nombre_invalido", "Escriba el nombre (hasta 120 caracteres).");
  if (esCorreoDeOperador(correo)) return error(400, "correo_invalido", "Ese correo es interno de los operadores.");
  if (await puertos.esAdministrador(correo)) return errorAdministrador();
  if (!(await puertos.insertar(correo, nombre))) return error(409, "ya_existe", "Ya hay un anfitrión con ese correo.");

  let cuenta: { id: string; creada: boolean } | null = null;
  try {
    cuenta = await puertos.asegurarCuenta(correo);
    await puertos.enlazar(correo, cuenta.id);
    const codigo = puertos.generarCodigo();
    const vence = await puertos.emitirCodigo(correo, codigo, admin, "crear");
    return { estado: 200, cuerpo: { correo, nombre, codigo, vence }, admin, accion: "crear" };
  } catch (fallo) {
    // Sin código el alta no sirve: se deshace para poder reintentar limpio.
    await puertos.borrar(correo).catch(() => {});
    if (cuenta?.creada) await puertos.borrarCuenta(cuenta.id).catch(() => {});
    return error(500, "error", fallo instanceof Error ? fallo.message : "No se pudo crear la cuenta.");
  }
}
