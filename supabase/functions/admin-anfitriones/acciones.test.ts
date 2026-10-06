import { assertEquals } from "jsr:@std/assert@1";
import { type FilaAnfitrion, procesar, type Puertos } from "./acciones.ts";

const ADMIN = "admin@gmail.example";

/** Base en memoria; registra cada llamada para verificar efectos. */
function puertosDePrueba(opciones: {
  admin?: string | null;
  administradores?: string[];
  filas?: FilaAnfitrion[];
  cuentasExistentes?: Record<string, string>;
  falloAlEmitir?: boolean;
} = {}) {
  const filas = new Map((opciones.filas ?? []).map((f) => [f.correo, { ...f }]));
  const cuentas = new Map(Object.entries(opciones.cuentasExistentes ?? {}));
  const llamadas: string[] = [];
  let siguiente = 1;
  const puertos: Puertos = {
    autorizar: () => Promise.resolve(opciones.admin === undefined ? ADMIN : opciones.admin),
    esAdministrador: (correo) => Promise.resolve((opciones.administradores ?? []).includes(correo)),
    buscar: (correo) => Promise.resolve(filas.get(correo) ?? null),
    insertar(correo, nombre) {
      if (filas.has(correo)) return Promise.resolve(false);
      filas.set(correo, { correo, nombre, activo: true, auth_user_id: null });
      llamadas.push(`insertar ${correo} ${nombre}`);
      return Promise.resolve(true);
    },
    borrar(correo) {
      filas.delete(correo);
      llamadas.push(`borrar ${correo}`);
      return Promise.resolve();
    },
    asegurarCuenta(correo) {
      const existente = cuentas.get(correo);
      if (existente) return Promise.resolve({ id: existente, creada: false });
      const id = `cuenta-${siguiente++}`;
      cuentas.set(correo, id);
      llamadas.push(`crearCuenta ${correo}`);
      return Promise.resolve({ id, creada: true });
    },
    borrarCuenta(id) {
      llamadas.push(`borrarCuenta ${id}`);
      return Promise.resolve();
    },
    enlazar(correo, id) {
      filas.get(correo)!.auth_user_id = id;
      return Promise.resolve();
    },
    contrasenaAleatoria(id) {
      llamadas.push(`aleatoria ${id}`);
      return Promise.resolve();
    },
    cerrarSesiones(id) {
      llamadas.push(`cerrarSesiones ${id}`);
      return Promise.resolve();
    },
    bloquear(id, bloquear) {
      llamadas.push(`${bloquear ? "bloquear" : "desbloquear"} ${id}`);
      return Promise.resolve();
    },
    generarCodigo: () => "ABCDE23456",
    emitirCodigo(correo, _codigo, admin, accion) {
      if (opciones.falloAlEmitir) return Promise.reject(new Error("sin base"));
      llamadas.push(`emitir ${correo} ${admin} ${accion}`);
      return Promise.resolve("2026-10-09T12:00:00Z");
    },
    cambiarEstado(correo, activo) {
      const fila = filas.get(correo)!;
      fila.activo = activo;
      llamadas.push(`estado ${correo} ${activo}`);
      return Promise.resolve(fila.auth_user_id);
    },
  };
  return { puertos, llamadas, filas };
}

Deno.test("solo un administrador del panel", async () => {
  const { puertos, llamadas } = puertosDePrueba({ admin: null });
  const r = await procesar({ accion: "crear", correo: "ana@empresa.example", nombre: "Ana" }, puertos);
  assertEquals(r.estado, 401);
  assertEquals(llamadas, []);
});

Deno.test("crear: normaliza, crea la cuenta, la enlaza y devuelve el código una vez", async () => {
  const { puertos, llamadas, filas } = puertosDePrueba();
  const r = await procesar({ accion: "crear", correo: "  Ana@Empresa.Example ", nombre: "ana mora" }, puertos);
  assertEquals(r.estado, 200);
  assertEquals(r.cuerpo, {
    correo: "ana@empresa.example",
    nombre: "ANA MORA",
    codigo: "ABCDE23456",
    vence: "2026-10-09T12:00:00Z",
  });
  assertEquals(filas.get("ana@empresa.example")?.auth_user_id, "cuenta-1");
  assertEquals(llamadas, [
    "insertar ana@empresa.example ANA MORA",
    "crearCuenta ana@empresa.example",
    `emitir ana@empresa.example ${ADMIN} crear`,
  ]);
});

Deno.test("crear: rechaza correos de administradores, de operadores y repetidos", async () => {
  const { puertos } = puertosDePrueba({
    administradores: ["jefe@gmail.example"],
    filas: [{ correo: "ana@empresa.example", nombre: "ANA", activo: true, auth_user_id: "x" }],
  });
  const crear = (correo: string) => procesar({ accion: "crear", correo, nombre: "X" }, puertos);
  assertEquals((await crear("jefe@gmail.example")).cuerpo, {
    error: "es_administrador",
    detail:
      "Ese correo es de un administrador del panel. Para anfitriones use otro correo: una contraseña no debe dar acceso al panel.",
  });
  assertEquals((await crear("110110110@brisas.local")).estado, 400);
  assertEquals((await crear("ana@empresa.example")).estado, 409);
  assertEquals((await crear("no-es-correo")).estado, 400);
});

Deno.test("crear: si falla el código, deshace el alta y la cuenta nueva", async () => {
  const { puertos, llamadas, filas } = puertosDePrueba({ falloAlEmitir: true });
  const r = await procesar({ accion: "crear", correo: "ana@empresa.example", nombre: "Ana" }, puertos);
  assertEquals(r.estado, 500);
  assertEquals(filas.has("ana@empresa.example"), false);
  assertEquals(llamadas.slice(-2), ["borrar ana@empresa.example", "borrarCuenta cuenta-1"]);
});

Deno.test("crear: reutiliza la cuenta de quien ya entraba con Google, sin borrarla si algo falla", async () => {
  const { puertos, llamadas } = puertosDePrueba({
    cuentasExistentes: { "ana@empresa.example": "google-1" },
    falloAlEmitir: true,
  });
  await procesar({ accion: "crear", correo: "ana@empresa.example", nombre: "Ana" }, puertos);
  assertEquals(llamadas.includes("borrarCuenta google-1"), false);
});

Deno.test("restablecer: invalida la contraseña, cierra sesiones y emite otro código", async () => {
  const { puertos, llamadas } = puertosDePrueba({
    filas: [{ correo: "ana@empresa.example", nombre: "ANA", activo: true, auth_user_id: "cuenta-9" }],
  });
  const r = await procesar({ accion: "restablecer", correo: "ana@empresa.example" }, puertos);
  assertEquals(r.estado, 200);
  assertEquals(llamadas, [
    "aleatoria cuenta-9",
    "cerrarSesiones cuenta-9",
    `emitir ana@empresa.example ${ADMIN} restablecer`,
  ]);
});

Deno.test("restablecer: no a una cuenta deshabilitada ni inexistente", async () => {
  const { puertos } = puertosDePrueba({
    filas: [{ correo: "ana@empresa.example", nombre: "ANA", activo: false, auth_user_id: "c" }],
  });
  assertEquals((await procesar({ accion: "restablecer", correo: "ana@empresa.example" }, puertos)).estado, 409);
  assertEquals((await procesar({ accion: "restablecer", correo: "otra@empresa.example" }, puertos)).estado, 404);
});

Deno.test("deshabilitar bloquea y cierra sesiones; habilitar desbloquea", async () => {
  const { puertos, llamadas } = puertosDePrueba({
    filas: [{ correo: "ana@empresa.example", nombre: "ANA", activo: true, auth_user_id: "cuenta-9" }],
  });
  await procesar({ accion: "deshabilitar", correo: "ana@empresa.example" }, puertos);
  await procesar({ accion: "habilitar", correo: "ana@empresa.example" }, puertos);
  assertEquals(llamadas, [
    "estado ana@empresa.example false",
    "bloquear cuenta-9",
    "cerrarSesiones cuenta-9",
    "estado ana@empresa.example true",
    "desbloquear cuenta-9",
  ]);
});

Deno.test("deshabilitar a quien también es administrador no le bloquea el panel", async () => {
  const { puertos, llamadas } = puertosDePrueba({
    administradores: ["jefe@gmail.example"],
    filas: [{ correo: "jefe@gmail.example", nombre: "JEFE", activo: true, auth_user_id: "cuenta-1" }],
  });
  await procesar({ accion: "deshabilitar", correo: "jefe@gmail.example" }, puertos);
  assertEquals(llamadas, ["estado jefe@gmail.example false"]);
});

Deno.test("acción desconocida", async () => {
  const { puertos } = puertosDePrueba();
  assertEquals((await procesar({ accion: "borrar", correo: "ana@empresa.example" }, puertos)).estado, 400);
});
