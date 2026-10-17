// Pruebas de la lógica pura del receptor. Con Node 22.18 o más:
//   node --test supabase/functions-staging/receptor-errores/logica.test.ts
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  calcularHuella,
  claveDeLaPeticion,
  claveValida,
  extraerEvento,
  leerEnvelope,
  limpiarTexto,
} from "./logica.ts";

const codificar = (texto: string) => new TextEncoder().encode(texto);

/** Un envelope con un solo evento, como lo arma el SDK (con `length`). */
function envelopeDe(evento: unknown, conLargo = true): Uint8Array {
  const cuerpo = JSON.stringify(evento);
  const largo = conLargo ? `,"length":${codificar(cuerpo).length}` : "";
  return codificar(
    `{"event_id":"9ec79c33ec9942ab8353589fcb2e04dc","sent_at":"2026-10-17T05:06:36Z"}\n` +
      `{"type":"event"${largo}}\n${cuerpo}\n`,
  );
}

const eventoRust = {
  event_id: "9ec79c33ec9942ab8353589fcb2e04dc",
  timestamp: 1792213596.858,
  level: "warning",
  release: "control-acceso-desktop@1.8.11",
  environment: "production",
  server_name: "PC-DE-PRUEBA",
  message: "sincronización automática falló: No hay una sesión activa",
  sdk: { name: "sentry.rust", version: "0.49.0" },
  user: { geo: { country_code: "CR", city: "San José" }, ip_address: "200.119.185.54" },
  contexts: { os: { name: "windows", version: "10.0.26300" }, device: { arch: "x86_64" } },
  breadcrumbs: { values: [{ message: "cédula 1-0123-0456" }] },
};

const eventoAndroid = {
  event_id: "11112222333344445555666677778888",
  timestamp: "2026-10-17T05:06:36.000Z",
  level: "fatal",
  release: "com.dqm27.lattis@1.4.13+35",
  sdk: { name: "sentry.java.android", version: "8.9.0" },
  exception: {
    values: [
      {
        type: "RejectedExecutionException",
        value: "Task rejected, pool size = 0, completed tasks = 5",
        stacktrace: {
          frames: [
            { function: "execute", module: "java.util.concurrent.ThreadPoolExecutor", in_app: false },
            { function: "sincronizar", module: "com.brisas.controlacceso.NubeViewModel", filename: "NubeViewModel.kt", lineno: 57, in_app: true },
          ],
        },
      },
    ],
  },
  contexts: { device: { model: "SM-A256E", arch: "arm64-v8a" } },
};

test("lee un envelope con length", () => {
  const envelope = leerEnvelope(envelopeDe(eventoRust));
  assert.ok(envelope);
  assert.equal(envelope.items.length, 1);
  assert.equal(envelope.items[0].cabecera.type, "event");
});

test("lee un envelope sin length (el contenido llega hasta el salto de línea)", () => {
  const envelope = leerEnvelope(envelopeDe(eventoRust, false));
  assert.equal(envelope?.items.length, 1);
  assert.ok(extraerEvento(envelope!.items[0].payload));
});

test("lee varios ítems y respeta length aunque el contenido tenga saltos de línea", () => {
  const cuerpo = '{"a":"uno\\ndos"}';
  const crudo = codificar(
    `{"event_id":"x"}\n{"type":"session","length":${cuerpo.length}}\n${cuerpo}\n{"type":"event"}\n{"event_id":"y"}\n`,
  );
  const envelope = leerEnvelope(crudo);
  assert.equal(envelope?.items.length, 2);
  assert.equal(envelope?.items[0].cabecera.type, "session");
  assert.equal(envelope?.items[1].cabecera.type, "event");
});

test("rechaza lo que no es un envelope", () => {
  assert.equal(leerEnvelope(codificar("hola")), null);
  assert.equal(leerEnvelope(codificar("")), null);
  assert.equal(leerEnvelope(codificar('{"event_id":"x"}\n{"type":"event","length":999}\n{}\n')), null);
});

test("un mensaje de Rust queda sin servidor, usuario ni rastro de pasos", () => {
  const evento = extraerEvento(codificar(JSON.stringify(eventoRust)));
  assert.ok(evento);
  assert.equal(evento.plataforma, "escritorio");
  assert.equal(evento.tipo, "mensaje");
  assert.equal(evento.titulo, "sincronización automática falló: No hay una sesión activa");
  assert.equal(evento.version_app, "control-acceso-desktop@1.8.11");
  assert.deepEqual(evento.contexto, { os: "windows", os_version: "10.0.26300", arquitectura: "x86_64" });
  const serializado = JSON.stringify(evento);
  for (const dato of ["PC-DE-PRUEBA", "200.119.185.54", "San José", "1-0123-0456"]) {
    assert.ok(!serializado.includes(dato), `no debe contener ${dato}`);
  }
});

test("una excepción de Android trae su traza y marca el código propio", () => {
  const evento = extraerEvento(codificar(JSON.stringify(eventoAndroid)));
  assert.ok(evento);
  assert.equal(evento.plataforma, "movil");
  assert.equal(evento.tipo, "excepcion");
  assert.equal(evento.nivel, "fatal");
  assert.match(evento.titulo, /^RejectedExecutionException: /);
  assert.equal(evento.traza.length, 2);
  assert.equal(evento.traza[1].propio, true);
  assert.equal(evento.contexto.modelo, "SM-A256E");
});

test("descarta un evento sin event_id o sin contenido", () => {
  assert.equal(extraerEvento(codificar("{}")), null);
  assert.equal(extraerEvento(codificar(JSON.stringify({ event_id: "a", message: "   " }))), null);
});

test("limpia datos personales", () => {
  const casos: Array<[string, string]> = [
    ["falló para juan@correo.com", "falló para <correo>"],
    ["cédula 1-0123-0456", "cédula <num>"],
    ["cédula 102340456", "cédula <num>"],
    ["DIMEX 155812345678", "DIMEX <num>"],
    ["llamar al 8888-8888", "llamar al <num>"],
    ["placa ABC123 y BCD-1234", "placa <placa> y <placa>"],
    ["C:\\Users\\usuario-de-prueba\\AppData\\Local\\x.db", "C:\\Users\\<usuario>\\AppData\\Local\\x.db"],
    ["/home/maria/app/datos", "/home/<usuario>/app/datos"],
    ["token eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiIxIn0.abc123def", "token <token>"],
    ["id 123e4567-e89b-12d3-a456-426614174000", "id <token>"],
    ["GET https://x.supabase.co/rest/v1/a?cedula=eq.123", "GET https://x.supabase.co/rest/v1/a?<datos>"],
  ];
  for (const [entrada, esperado] of casos) assert.equal(limpiarTexto(entrada), esperado, entrada);
});

test("no toca lo que no es personal", () => {
  const texto = "No se pudo conectar con la nube, intentá de nuevo (pool size = 0, 5 tareas)";
  assert.equal(limpiarTexto(texto), texto);
});

test("la huella agrupa lo mismo aunque cambien los números", async () => {
  const a = extraerEvento(codificar(JSON.stringify({ ...eventoAndroid, event_id: "a".repeat(32) })))!;
  const otro = structuredClone(eventoAndroid);
  otro.exception.values[0].value = "Task rejected, pool size = 7, completed tasks = 912";
  const b = extraerEvento(codificar(JSON.stringify({ ...otro, event_id: "b".repeat(32) })))!;
  assert.equal(await calcularHuella(a), await calcularHuella(b));
});

test("la huella separa errores distintos y plataformas distintas", async () => {
  const android = extraerEvento(codificar(JSON.stringify(eventoAndroid)))!;
  const rust = extraerEvento(codificar(JSON.stringify(eventoRust)))!;
  const otroMensaje = extraerEvento(
    codificar(JSON.stringify({ ...eventoRust, message: "sincronización automática falló: otra cosa" })),
  )!;
  const huellas = new Set([
    await calcularHuella(android),
    await calcularHuella(rust),
    await calcularHuella(otroMensaje),
  ]);
  assert.equal(huellas.size, 3);
  assert.equal((await calcularHuella(rust)).length, 16);
});

test("lee la clave del DSN del encabezado o de la query", () => {
  const url = new URL("https://x.supabase.co/functions/v1/receptor-errores/api/1/envelope/");
  const conEncabezado = new Headers({ "x-sentry-auth": "Sentry sentry_key=abc123, sentry_version=7" });
  assert.equal(claveDeLaPeticion(conEncabezado, url), "abc123");
  assert.equal(claveDeLaPeticion(new Headers(), new URL(`${url}?sentry_key=zzz`)), "zzz");
  assert.equal(claveDeLaPeticion(new Headers(), url), null);
});

test("compara la clave sin aceptar vacíos ni largos distintos", () => {
  assert.equal(claveValida("abc123", "abc123"), true);
  assert.equal(claveValida("abc124", "abc123"), false);
  assert.equal(claveValida("abc", "abc123"), false);
  assert.equal(claveValida(null, "abc123"), false);
  assert.equal(claveValida("abc123", ""), false);
});
