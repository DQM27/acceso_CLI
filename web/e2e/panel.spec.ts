import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

const correo = "admin@example.invalid";
const usuario = {
  id: "00000000-0000-4000-8000-000000000001",
  aud: "authenticated",
  role: "authenticated",
  email: correo,
  app_metadata: { provider: "google", providers: ["google"] },
  user_metadata: { full_name: "Admin de prueba" },
  created_at: "2026-09-09T12:00:00Z",
};
const sitio = {
  id: "00000000-0000-4000-8000-000000000002",
  nombre: "Brisas",
  created_at: "2026-09-09T12:00:00Z",
};

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const estado = window as unknown as { violacionesCsp: string[] };
    estado.violacionesCsp = [];
    document.addEventListener("securitypolicyviolation", (evento) =>
      estado.violacionesCsp.push(
        `${evento.violatedDirective}: ${evento.blockedURI} @ ${evento.sourceFile}:${evento.lineNumber}:${evento.columnNumber}`,
      ),
    );
  });
});
test.afterEach(async ({ page }) => {
  const violaciones = await page.evaluate(
    () => (window as unknown as { violacionesCsp: string[] }).violacionesCsp,
  );
  expect(violaciones ?? []).toEqual([]);
});

async function preparar(page: Page) {
  await page.addInitScript(
    ({ usuario }) => {
      const ahora = Math.floor(Date.now() / 1000);
      localStorage.setItem(
        "sb-xidaepyaljzkpbsxrqsm-auth-token",
        JSON.stringify({
          access_token: `prueba.${btoa(JSON.stringify({ sub: usuario.id, exp: ahora + 3600 }))}.firma`,
          refresh_token: "solo-prueba",
          token_type: "bearer",
          expires_in: 3600,
          expires_at: ahora + 3600,
          user: usuario,
        }),
      );
    },
    { usuario },
  );
  await page.route("https://xidaepyaljzkpbsxrqsm.supabase.co/**", async (ruta) => {
    const peticion = ruta.request();
    const url = new URL(peticion.url());
    const responder = (datos: unknown, status = 200) =>
      ruta.fulfill({ status, contentType: "application/json", body: JSON.stringify(datos) });

    if (url.pathname === "/auth/v1/user") return responder(usuario);
    if (url.pathname === "/auth/v1/logout") return responder({});
    if (url.pathname === "/rest/v1/administradores_panel") return responder({ correo });
    if (url.pathname === "/rest/v1/contratistas")
      return responder([
        {
          id: "1",
          identificacion: "1-2345-6789",
          nombre: "Contratista de prueba",
          empresa_nombre: "Empresa de prueba",
          tipo_ingreso: "OBRA",
          fecha_vencimiento_praind: null,
          es_personal_ruta: false,
          activo: true,
        },
      ]);
    if (url.pathname === "/rest/v1/usuarios")
      return responder([
        { id: "1", cedula: "1-2345-6789", nombre: "Operador de prueba", rol: "OPERADOR", activo: true },
      ]);
    if (url.pathname === "/rest/v1/sitios") return responder([sitio]);
    if (url.pathname === "/rest/v1/ingresos")
      return responder([
        {
          id: "1",
          sitio_id: sitio.id,
          contratista_cedula: "1-2345-6789",
          contratista_nombre: "Contratista de prueba",
          empresa_nombre: "Empresa de prueba",
          tipo_ingreso: "OBRA",
          medio_ingreso: "MANUAL",
          gafete_numero: 12,
          hora_entrada: "2026-09-09T12:00:00Z",
          hora_salida: "2026-09-09T18:00:00Z",
          usuario_entrada_nombre: "Operador de prueba",
          usuario_salida_nombre: "Operador de prueba",
          sitios: { nombre: sitio.nombre },
          dispositivo_entrada: { tipo: "pc" },
        },
      ]);
    if (url.pathname === "/functions/v1/admin-list-devices")
      return responder({
        sitios: [sitio],
        dispositivos: [
          {
            id: "3",
            sitio_id: sitio.id,
            tipo: "pc",
            etiqueta: "PC de prueba",
            created_at: "2026-09-09T12:00:00Z",
            revoked_at: null,
            suspended_at: null,
            last_seen_at: null,
            oculto_en_panel: false,
            identificador_hardware: null,
            nombre_dispositivo: null,
            plataforma: null,
            version_build: null,
            app_version: null,
            last_ip: null,
          },
        ],
      });
    throw new Error(`Petición inesperada: ${url.pathname}`);
  });
}

test("Contratistas (AG Grid) carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/contratistas");
  await expect(page.getByText("Contratista de prueba")).toBeVisible();
});

test("Usuarios (AG Grid) carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/usuarios");
  await expect(page.getByText("Operador de prueba")).toBeVisible();
});

test("Dispositivos carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/dispositivos");
  await expect(page.getByText("Brisas")).toBeVisible();
});

test("Historial (AG Grid) y exportación a PDF sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/historial");
  await expect(page.getByText("Contratista de prueba")).toBeVisible();

  // El punto de mayor riesgo real: exportarAPdf() escribe HTML crudo con un
  // <style> inline en un iframe oculto (about:blank hereda el CSP del
  // documento que lo creó) -- si algo en la política bloquea ese estilo,
  // tiene que aparecer acá como violación real, no sólo en teoría. La
  // violación (si la hay) se dispara de forma síncrona al escribir el HTML,
  // antes de que `print()` haga nada -- no hace falta esperar un diálogo.
  await page.getByRole("button", { name: /exportar a pdf/i }).click();
  await page.waitForTimeout(500);
});
