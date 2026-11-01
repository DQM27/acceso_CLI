import { assert, assertEquals } from "jsr:@std/assert@1";
import { armarLote, calcularHuella, extraerReporte, limpiarRuta, limpiarTexto, MAX_REPORTES } from "./errores.ts";

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

Deno.test("un pánico del escritorio queda con su traza al revés, sus operaciones y sin datos de más", () => {
  const error = extraerReporte(reportePanico);
  assert(error);
  assertEquals(error.event_id, reportePanico.id);
  assertEquals(error.origen, "panico");
  assertEquals(error.nivel, "fatal");
  assertEquals(error.version_app, "desktop-1.10.6+abc1234");
  assertEquals(error.pantalla, "Ingresos");
  assertEquals(error.ocurrido_en, "2026-10-31T12:00:00.123Z");
  // Del más viejo al más nuevo: el lugar del pánico queda al final.
  assertEquals(error.traza.at(-1), { funcion: null, archivo: "src/nube/mod.rs", linea: 120, propio: true });
  assertEquals(error.traza.length, 3);
  // Una operación con datos en el nombre no pasa.
  assertEquals(error.operaciones, [{ nombre: "buscar_contratista", ok: true, en: "2026-10-31T11:59:58.000Z" }]);
  // Sólo las claves de contexto de la lista.
  assertEquals(error.contexto, { os: "windows", arquitectura: "x86_64" });
});

Deno.test("una excepción de Kotlin limpia el título", () => {
  const error = extraerReporte(reporteKotlin);
  assert(error);
  assertEquals(error.titulo, "java.lang.IllegalStateException: cédula <num> de <correo>");
  assertEquals(error.contexto.ambiente, "produccion");
  assertEquals(error.nivel, "fatal");
});

Deno.test("un error de JavaScript o del núcleo es nivel error, no fatal", () => {
  assertEquals(extraerReporte({ ...reportePanico, origen: "js" })?.nivel, "error");
  assertEquals(extraerReporte({ ...reportePanico, origen: "nucleo" })?.nivel, "error");
});

Deno.test("descarta lo que no es un reporte", () => {
  assertEquals(extraerReporte(null), null);
  assertEquals(extraerReporte([]), null);
  assertEquals(extraerReporte({ ...reportePanico, id: "corto" }), null);
  assertEquals(extraerReporte({ ...reportePanico, id: "con espacios y ; raros" }), null);
  assertEquals(extraerReporte({ ...reportePanico, origen: "otro" }), null);
  assertEquals(extraerReporte({ ...reportePanico, titulo: "   " }), null);
});

Deno.test("una versión desconocida no se guarda tal cual", () => {
  assertEquals(extraerReporte({ ...reportePanico, version_app: "<script>" })?.version_app, null);
});

Deno.test("limpia datos personales", () => {
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
  for (const [entrada, esperado] of casos) assertEquals(limpiarTexto(entrada), esperado, entrada);
});

Deno.test("no toca lo que no es personal", () => {
  const texto = "No se pudo conectar con la nube, intente de nuevo (pool size = 0, 5 tareas)";
  assertEquals(limpiarTexto(texto), texto);
});

Deno.test("la ruta de la traza sólo pierde el usuario", () => {
  assertEquals(limpiarRuta("C:\\Users\\daniel\\.cargo\\x.rs"), "C:\\Users\\<usuario>\\.cargo\\x.rs");
  const propia = "desktop/src-tauri/src/comandos/nube.rs";
  assertEquals(limpiarRuta(propia), propia);
  const error = extraerReporte({ ...reportePanico, traza: [{ funcion: "x", archivo: propia, linea: 1, propio: true }] });
  assertEquals(error?.traza[0].archivo, propia);
});

Deno.test("la huella agrupa lo mismo aunque cambien los números", async () => {
  const a = extraerReporte({ ...reporteKotlin, titulo: "Timeout tras 1500 ms" })!;
  const b = extraerReporte({ ...reporteKotlin, id: "99998888-7777-6666-5555-444433332222", titulo: "Timeout tras 30 ms" })!;
  assertEquals(await calcularHuella(a), await calcularHuella(b));
});

Deno.test("la huella separa errores, orígenes y plataformas distintos", async () => {
  const huellas = new Set([
    await calcularHuella(extraerReporte(reportePanico)!),
    await calcularHuella(extraerReporte(reporteKotlin)!),
    await calcularHuella(extraerReporte({ ...reportePanico, origen: "nucleo" })!),
    await calcularHuella(extraerReporte({ ...reportePanico, titulo: "panic: otra cosa" })!),
  ]);
  assertEquals(huellas.size, 4);
});

Deno.test("el lote para la base: huella de 16 hexadecimales, sin plataforma, y los inválidos se cuentan", async () => {
  const lote = await armarLote({ reportes: [reportePanico, { ...reporteKotlin, origen: "otro" }, reporteKotlin, null] });
  assert(lote.ok);
  assertEquals(lote.descartados, 2);
  assertEquals(lote.errores.map((e) => e.event_id), [reportePanico.id, reporteKotlin.id]);
  for (const error of lote.errores) {
    assert(/^[0-9a-f]{16}$/.test(error.huella), error.huella);
    assert(!("plataforma" in error));
  }
  assertEquals(lote.errores[1].titulo, "java.lang.IllegalStateException: cédula <num> de <correo>");
});

Deno.test("un cuerpo sin lista o con demasiados reportes se rechaza entero", async () => {
  assertEquals(await armarLote({}), { ok: false, estado: 400, error: "sin_reportes" });
  assertEquals(await armarLote({ reportes: {} }), { ok: false, estado: 400, error: "sin_reportes" });
  const muchos = Array.from({ length: MAX_REPORTES + 1 }, () => reportePanico);
  assertEquals(await armarLote({ reportes: muchos }), { ok: false, estado: 400, error: "demasiados_reportes" });
  const justos = await armarLote({ reportes: Array.from({ length: MAX_REPORTES }, () => reportePanico) });
  assert(justos.ok);
});

Deno.test("una lista vacía igual llega a la base (que verifica el token)", async () => {
  assertEquals(await armarLote({ reportes: [] }), { ok: true, errores: [], descartados: 0 });
});
