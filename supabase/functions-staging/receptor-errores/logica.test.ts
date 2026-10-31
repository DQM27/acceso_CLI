// Pruebas de la lógica pura del receptor. Con Node 22.18 o más:
//   node --test supabase/functions-staging/receptor-errores/logica.test.ts
import assert from "node:assert/strict";
import { test } from "node:test";
import { calcularHuella, claveValida, extraerReporte, limpiarRuta, limpiarTexto } from "./logica.ts";

/** Un reporte como lo arma `control_acceso::errores` (traza del más nuevo al más viejo). */
const reportePanico = {
  esquema: 1,
  id: "550e8400-e29b-41d4-a716-446655440000",
  ocurrido_en: "2026-10-31T12:00:00.123Z",
  plataforma: "escritorio",
  version_app: "desktop-1.10.6+abc1234",
  origen: "panico",
  titulo: "panic: called `Option::unwrap()` on a `None` value",
  traza: [
    { funcion: null, archivo: "src/nube/mod.rs", linea: 120, propio: true },
    { funcion: "control_acceso::nube::sincronizar", archivo: "./src/nube/mod.rs", linea: 118, propio: true },
    { funcion: "tokio::runtime::task::harness::poll", archivo: null, linea: null, propio: false },
  ],
  pantalla: "Ingresos",
  operaciones: [
    { nombre: "buscar_contratista", ok: true, en: "2026-10-31T11:59:58.000Z" },
    { nombre: "registrar ingreso 1-0123-0456", ok: false, en: "2026-10-31T11:59:59.000Z" },
  ],
  contexto: { os: "windows", arquitectura: "x86_64", servidor: "PC-DE-PRUEBA" },
};

const reporteKotlin = {
  ...reportePanico,
  id: "11112222-3333-4444-5555-666677778888",
  plataforma: "movil",
  version_app: "1.8.12 (57)",
  origen: "excepcion",
  titulo: "java.lang.IllegalStateException: cédula 1-0123-0456 de ana@correo.com",
  traza: [
    { funcion: "com.brisas.controlacceso.Ingreso.guardar", archivo: "Ingreso.kt", linea: 88, propio: true },
    { funcion: "android.os.Handler.dispatchMessage", archivo: "Handler.java", linea: 106, propio: false },
  ],
  contexto: { modelo: "SM-A256E", ambiente: "produccion" },
};

test("un pánico del escritorio queda con su traza al revés, sus operaciones y sin datos de más", () => {
  const evento = extraerReporte(reportePanico);
  assert.ok(evento);
  assert.equal(evento.event_id, reportePanico.id);
  assert.equal(evento.plataforma, "escritorio");
  assert.equal(evento.origen, "panico");
  assert.equal(evento.nivel, "fatal");
  assert.equal(evento.version_app, "desktop-1.10.6+abc1234");
  assert.equal(evento.pantalla, "Ingresos");
  assert.equal(evento.ocurrido_en, "2026-10-31T12:00:00.123Z");
  // Del más viejo al más nuevo: el lugar del pánico queda al final.
  assert.deepEqual(evento.traza.at(-1), { funcion: null, archivo: "src/nube/mod.rs", linea: 120, propio: true });
  assert.equal(evento.traza.length, 3);
  // Una operación con datos en el nombre no pasa.
  assert.deepEqual(evento.operaciones, [{ nombre: "buscar_contratista", ok: true, en: "2026-10-31T11:59:58.000Z" }]);
  // Sólo las claves de contexto de la lista.
  assert.deepEqual(evento.contexto, { os: "windows", arquitectura: "x86_64" });
});

test("una excepción de Kotlin limpia el título y toma el ambiente como entorno", () => {
  const evento = extraerReporte(reporteKotlin);
  assert.ok(evento);
  assert.equal(evento.plataforma, "movil");
  assert.equal(evento.titulo, "java.lang.IllegalStateException: cédula <num> de <correo>");
  assert.equal(evento.entorno, "produccion");
  assert.equal(evento.nivel, "fatal");
});

test("un error de JavaScript o del núcleo es nivel error, no fatal", () => {
  assert.equal(extraerReporte({ ...reportePanico, origen: "js" })?.nivel, "error");
  assert.equal(extraerReporte({ ...reportePanico, origen: "nucleo" })?.nivel, "error");
});

test("descarta lo que no es un reporte", () => {
  assert.equal(extraerReporte(null), null);
  assert.equal(extraerReporte([]), null);
  assert.equal(extraerReporte({ ...reportePanico, id: "corto" }), null);
  assert.equal(extraerReporte({ ...reportePanico, origen: "otro" }), null);
  assert.equal(extraerReporte({ ...reportePanico, titulo: "   " }), null);
});

test("una plataforma o versión desconocida no se guarda tal cual", () => {
  const evento = extraerReporte({ ...reportePanico, plataforma: "ios", version_app: "<script>" });
  assert.equal(evento?.plataforma, "otra");
  assert.equal(evento?.version_app, null);
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
  const texto = "No se pudo conectar con la nube, intente de nuevo (pool size = 0, 5 tareas)";
  assert.equal(limpiarTexto(texto), texto);
});

test("la ruta de la traza sólo pierde el usuario", () => {
  assert.equal(limpiarRuta("C:\\Users\\daniel\\.cargo\\x.rs"), "C:\\Users\\<usuario>\\.cargo\\x.rs");
  const propia = "desktop/src-tauri/src/comandos/nube.rs";
  assert.equal(limpiarRuta(propia), propia);
  const evento = extraerReporte({
    ...reportePanico,
    traza: [{ funcion: "x", archivo: propia, linea: 1, propio: true }],
  });
  assert.equal(evento?.traza[0].archivo, propia);
});

test("la huella agrupa lo mismo aunque cambien los números", async () => {
  const a = extraerReporte({ ...reporteKotlin, titulo: "Timeout tras 1500 ms" })!;
  const b = extraerReporte({ ...reporteKotlin, id: "99998888-7777-6666-5555-444433332222", titulo: "Timeout tras 30 ms" })!;
  assert.equal(await calcularHuella(a), await calcularHuella(b));
});

test("la huella separa errores, orígenes y plataformas distintos", async () => {
  const huellas = new Set([
    await calcularHuella(extraerReporte(reportePanico)!),
    await calcularHuella(extraerReporte(reporteKotlin)!),
    await calcularHuella(extraerReporte({ ...reportePanico, origen: "nucleo" })!),
    await calcularHuella(extraerReporte({ ...reportePanico, titulo: "panic: otra cosa" })!),
  ]);
  assert.equal(huellas.size, 4);
  assert.equal((await calcularHuella(extraerReporte(reportePanico)!)).length, 16);
});

test("compara la clave sin aceptar vacíos ni largos distintos", () => {
  assert.equal(claveValida("abc123", "abc123"), true);
  assert.equal(claveValida("abc124", "abc123"), false);
  assert.equal(claveValida("abc", "abc123"), false);
  assert.equal(claveValida(null, "abc123"), false);
  assert.equal(claveValida("abc123", ""), false);
});
