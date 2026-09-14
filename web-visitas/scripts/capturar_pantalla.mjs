/**
 * Captura una pantalla de web-visitas con Playwright, con sesión y datos de
 * Supabase mockeados -- para verificar visualmente un cambio sin depender
 * de un navegador real ni de datos de producción. Requiere `npm run dev`
 * (o `npm run preview`) corriendo en otra terminal.
 *
 * Uso: `node scripts/capturar_pantalla.mjs` desde `web-visitas/`, con
 * variables de entorno:
 *   URL=http://127.0.0.1:5174/nueva   página a visitar (default: /citas)
 *   OUT=captura.png                    archivo de salida
 *   THEME=light|dark                   tema (default: light)
 *   ANCHO=1280  ALTO=900                viewport
 *   CON_CITA=1                          agrega una cita VIGENTE de prueba
 *   CLICK_CALENDARIO=1                  click en el toggle Calendario (Mis Citas)
 *   CLICK_DETALLE=1                     abre el modal de detalle de la primera cita
 *   CLICK_CANCELAR=1                    abre el modal de cancelar la primera cita
 *   LLENAR_Y_REVISAR=1                  en /nueva: elige sitio y avanza a Visitantes
 *   SOLO_PASO2=1                        con LLENAR_Y_REVISAR, se queda en el paso 2
 *                                       (no llena ni avanza a Confirmar)
 *
 * Ejemplo: `URL=http://127.0.0.1:5174/nueva OUT=paso1.png THEME=dark node scripts/capturar_pantalla.mjs`
 */
import { chromium } from "playwright";

const url = process.env.URL ?? "http://127.0.0.1:5174/citas";
const out = process.env.OUT ?? "captura.png";
const theme = process.env.THEME ?? "light";
const conCita = process.env.CON_CITA === "1";
const clickDetalle = process.env.CLICK_DETALLE === "1";
const clickCancelar = process.env.CLICK_CANCELAR === "1";

const uuid = "00000000-0000-4000-8000-000000000001";
const correo = "anfitrion@example.invalid";
const usuario = {
  id: uuid,
  aud: "authenticated",
  role: "authenticated",
  email: correo,
  app_metadata: { provider: "google", providers: ["google"] },
  user_metadata: {},
  created_at: "2026-09-09T12:00:00Z",
};
const sitios = [
  { id: uuid, nombre: "Brisas" },
  { id: "00000000-0000-4000-8000-000000000002", nombre: "Cartago" },
];
const citas = conCita
  ? [
      {
        id: uuid,
        anfitrion_correo: correo,
        motivo: "Reunión de coordinación",
        fecha_desde: "2099-09-10",
        fecha_hasta: "2099-09-11",
        hora_estimada: "10:00:00",
        estado: "VIGENTE",
        created_at: "2026-09-09T12:00:00Z",
        cita_sitios: [{ sitio_id: uuid, sitios: sitios[0] }],
        cita_visitantes: [
          {
            id: uuid,
            nombre: "Persona de prueba",
            cedula: "DOC123",
            empresa: "Empresa de prueba",
            placa_vehiculo: null,
          },
        ],
      },
    ]
  : [];

const browser = await chromium.launch();
const alto = Number(process.env.ALTO ?? 900);
const ancho = Number(process.env.ANCHO ?? 1280);
const page = await browser.newPage({ viewport: { width: ancho, height: alto } });

await page.addInitScript(
  ({ usuario }) => {
    const token = `prueba.${btoa(JSON.stringify({ sub: usuario.id, exp: Math.floor(Date.now() / 1000) + 3600 }))}.firma`;
    sessionStorage.setItem(
      "brisas-visitas-auth",
      JSON.stringify({
        access_token: token,
        refresh_token: "solo-prueba",
        token_type: "bearer",
        expires_in: 3600,
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        user: usuario,
      }),
    );
  },
  { usuario },
);
await page.addInitScript((theme) => {
  try {
    localStorage.setItem("brisas:tema", theme);
  } catch {}
}, theme);

await page.route("https://xidaepyaljzkpbsxrqsm.supabase.co/**", async (ruta) => {
  const u = new URL(ruta.request().url());
  console.log("[route]", ruta.request().method(), u.pathname, u.search);
  const responder = (datos, status = 200) =>
    ruta.fulfill({ status, contentType: "application/json", body: JSON.stringify(datos) });
  if (u.pathname === "/auth/v1/user") return responder(usuario);
  if (u.pathname === "/auth/v1/logout") return responder({});
  if (u.pathname === "/rest/v1/anfitriones")
    return responder({ correo, nombre: "Daniel · Cuenta de prueba" });
  if (u.pathname === "/rest/v1/sitios") return responder(sitios);
  if (u.pathname === "/rest/v1/citas") return responder(citas);
  return responder({}, 200);
});

page.on("console", (m) => console.log("[console]", m.text()));
page.on("pageerror", (e) => console.log("[pageerror]", e.stack ?? e.message));
page.on("requestfailed", (r) => console.log("[requestfailed]", r.url(), r.failure()?.errorText));

await page.goto(url, { waitUntil: "networkidle" });
await page.waitForTimeout(2000);
if (process.env.CLICK_CALENDARIO === "1") {
  await page.getByRole("button", { name: /Calendario/i }).click();
  await page.waitForTimeout(500);
}
if (clickDetalle) {
  await page.getByRole("button", { name: /Ver detalles/i }).first().click();
  await page.waitForTimeout(300);
}
if (process.env.LLENAR_Y_REVISAR === "1") {
  // El checkbox de sitio va oculto a propósito (patrón .btn-check de
  // Bootstrap, pointer-events:none) -- hay que clickear la etiqueta
  // visible, no el input, igual que hace un usuario real con mouse.
  await page.locator(".selector-sitios").getByText("Brisas", { exact: true }).click();
  await page.getByRole("button", { name: /Continuar/i }).click();
  await page.waitForTimeout(200);
  if (process.env.SOLO_PASO2 !== "1") {
    await page.getByLabel(/Nombre completo/).fill("Visitante de prueba");
    await page.getByLabel(/Cédula o documento/).fill("1-2345-6789");
    await page.getByRole("button", { name: /Continuar/i }).click();
    await page.waitForTimeout(300);
  }
}
if (clickCancelar) {
  await page.getByRole("button", { name: /Cancelar cita/i }).first().click();
  await page.waitForTimeout(300);
}
await page.screenshot({ path: out, fullPage: false });
await browser.close();
console.log("OK", out);
