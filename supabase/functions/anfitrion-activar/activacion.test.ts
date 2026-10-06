import { assertEquals } from "jsr:@std/assert@1";
import { activar, CODIGO_INVALIDO, type Puertos } from "./activacion.ts";

const CONTRASENA = "el cafe de la tarde en cartago";

function puertosDePrueba(opciones: { valido?: boolean; debil?: boolean } = {}) {
  const llamadas: string[] = [];
  const puertos: Puertos = {
    verificar(correo, codigo) {
      llamadas.push(`verificar ${correo} ${codigo}`);
      return Promise.resolve(opciones.valido === false ? null : "cuenta-1");
    },
    ponerContrasena(id) {
      llamadas.push(`contrasena ${id}`);
      return Promise.resolve(opciones.debil ? "debil" : "ok");
    },
    consumir(correo) {
      llamadas.push(`consumir ${correo}`);
      return Promise.resolve();
    },
    cerrarSesiones(id) {
      llamadas.push(`cerrarSesiones ${id}`);
      return Promise.resolve();
    },
  };
  return { puertos, llamadas };
}

Deno.test("activa: normaliza correo y código, guarda, consume y cierra sesiones", async () => {
  const { puertos, llamadas } = puertosDePrueba();
  const r = await activar({ correo: " Ana@Empresa.Example", codigo: "abcde-23456", contrasena: CONTRASENA }, puertos);
  assertEquals(r, { estado: 200, cuerpo: { ok: true } });
  assertEquals(llamadas, [
    "verificar ana@empresa.example ABCDE23456",
    "contrasena cuenta-1",
    "consumir ana@empresa.example",
    "cerrarSesiones cuenta-1",
  ]);
});

Deno.test("un código rechazado da siempre el mismo mensaje", async () => {
  const { puertos, llamadas } = puertosDePrueba({ valido: false });
  const r = await activar({ correo: "ana@empresa.example", codigo: "ABCDE23456", contrasena: CONTRASENA }, puertos);
  assertEquals(r, { estado: 400, cuerpo: CODIGO_INVALIDO });
  assertEquals(llamadas.length, 1);
});

Deno.test("un código con forma imposible ni siquiera consulta la base", async () => {
  const { puertos, llamadas } = puertosDePrueba();
  for (const codigo of ["", "ABC", "ABCDE2345O", 12345]) {
    const r = await activar({ correo: "ana@empresa.example", codigo, contrasena: CONTRASENA }, puertos);
    assertEquals(r, { estado: 400, cuerpo: CODIGO_INVALIDO });
  }
  assertEquals(llamadas, []);
});

Deno.test("una contraseña corta no gasta un intento del código", async () => {
  const { puertos, llamadas } = puertosDePrueba();
  const r = await activar({ correo: "ana@empresa.example", codigo: "ABCDE23456", contrasena: "corta" }, puertos);
  assertEquals(r.estado, 422);
  assertEquals(llamadas, []);
});

Deno.test("si Supabase rechaza la contraseña, el código no se consume", async () => {
  const { puertos, llamadas } = puertosDePrueba({ debil: true });
  const r = await activar({ correo: "ana@empresa.example", codigo: "ABCDE23456", contrasena: CONTRASENA }, puertos);
  assertEquals(r.estado, 422);
  assertEquals(llamadas.includes("consumir ana@empresa.example"), false);
});

Deno.test("no acepta el usuario del correo dentro de la contraseña", async () => {
  const { puertos } = puertosDePrueba();
  const r = await activar(
    { correo: "ana.mora@empresa.example", codigo: "ABCDE23456", contrasena: "ana.mora visita la planta" },
    puertos,
  );
  assertEquals(r.estado, 422);
});
