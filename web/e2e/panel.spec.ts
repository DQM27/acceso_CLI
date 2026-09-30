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
    if (url.pathname === "/rest/v1/empresas") return responder([{ id: "e1", nombre: "EMPRESA DE PRUEBA" }]);
    if (url.pathname === "/rest/v1/rpc/panel_crear_contratista")
      return responder({
        id: "9",
        identificacion: "112340567",
        nombre: "ANA PEREZ",
        empresa_nombre: "EMPRESA DE PRUEBA",
        tipo_ingreso: "PRAIND",
        fecha_vencimiento_praind: null,
        es_personal_ruta: false,
        activo: false,
      });
    if (url.pathname === "/rest/v1/panel_contratistas_estado")
      return responder([
        {
          id: "1",
          identificacion: "1-2345-6789",
          nombre: "Contratista de prueba",
          empresa_nombre: "Empresa de prueba",
          tipo_ingreso: "PRAIND",
          fecha_vencimiento_praind: "2026-09-01",
          es_personal_ruta: false,
          activo: true,
          empresa_activa: true,
          requiere_praind: true,
          dias_para_vencer: -3,
          estado_praind: "VENCIDA",
          estado_acceso: "PRAIND_VENCIDO",
          adentro_sitio_nombre: sitio.nombre,
          adentro_desde: "2026-09-30T14:00:00Z",
        },
      ]);
    if (url.pathname === "/rest/v1/panel_adentro_ahora")
      return responder([
        {
          tipo: "PROVEEDOR",
          id: "p1",
          sitio_id: sitio.id,
          sitio_nombre: sitio.nombre,
          identificacion: "800000001",
          nombre: "Proveedor de prueba",
          empresa_nombre: "Distribuidora",
          gafete_numero: 31,
          placa: "ABC123",
          hora_entrada: new Date(Date.now() - 90 * 60_000).toISOString(),
          usuario_entrada_nombre: "Operador de prueba",
        },
      ]);
    if (url.pathname === "/rest/v1/usuarios")
      return responder([
        { id: "1", cedula: "1-2345-6789", nombre: "Operador de prueba", rol: "OPERADOR", activo: true },
      ]);
    if (url.pathname === "/rest/v1/sitios") return responder([sitio]);
    if (url.pathname === "/rest/v1/panel_movimientos")
      return responder([
        {
          id: "1",
          sitio_id: sitio.id,
          sitio_nombre: sitio.nombre,
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
          dispositivo_entrada_tipo: "pc",
          tipo_texto: "OBRA",
          medio_texto: "CAMINANDO",
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
            last_seen_at: null,
            oculto_en_panel: false,
            identificador_hardware: null,
            nombre_dispositivo: null,
            plataforma: null,
            version_build: null,
            app_version: null,
            last_ip: null,
            clave_huella: "huella-prueba",
            vinculado_en: "2026-09-09T12:05:00Z",
            credencial: "clave",
          },
        ],
        codigos_pendientes: [],
        eventos: [
          {
            id: 1,
            ocurrido_en: "2026-09-09T12:10:00Z",
            dispositivo_id: "3",
            tipo: "codigo_usado",
            ip: "203.0.113.7",
            detalle: null,
          },
        ],
      });
    if (url.pathname === "/functions/v1/admin-provision-device")
      return responder({
        sitio_id: sitio.id,
        sitio_nombre: sitio.nombre,
        dispositivo_id: "4",
        codigo: "K7QM-R4XT-2P",
        expira_en: new Date(Date.now() + 15 * 60_000).toISOString(),
      });
    throw new Error(`Petición inesperada: ${url.pathname}`);
  });
}

test("Contratistas (AG Grid) carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/contratistas");
  await expect(page.getByText("Contratista de prueba")).toBeVisible();
});

test("Adentro ahora muestra quién está adentro y el conteo por tipo", async ({ page }) => {
  await preparar(page);
  await page.goto("/adentro");
  await expect(page.getByText("Proveedor de prueba")).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "1 adentro" })).toContainText("Proveedor: 1");
});

test("Contratistas: el modal registra a alguien con el acceso denegado", async ({ page }) => {
  await preparar(page);
  await page.goto("/contratistas");
  await page.getByRole("button", { name: "+ Nuevo" }).click();

  const modal = page.getByRole("dialog");
  await expect(modal.getByText("Nuevo contratista")).toBeVisible();
  await expect(modal.getByText("Fecha de vencimiento PRAIND")).toBeVisible();
  // Denegar el acceso quita el PRAIND: a quien no va a entrar no se le pide.
  await modal.getByLabel("Crear con el acceso denegado").check();
  await expect(modal.getByText("Fecha de vencimiento PRAIND")).toBeHidden();

  await modal.getByLabel("Cédula").fill("1-1234-0567");
  await modal.getByLabel("Nombre").fill("Ana Perez");
  await modal.getByLabel("Empresa", { exact: false }).first().selectOption("e1");
  await modal.getByRole("button", { name: "Guardar" }).click();

  await expect(page.getByText("ANA PEREZ registrado con el acceso denegado.")).toBeVisible();
  await expect(modal).toBeHidden();
});

test("Usuarios (AG Grid) carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/usuarios");
  await expect(page.getByText("Operador de prueba")).toBeVisible();
});

test("Dispositivos carga sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/dispositivos");
  await expect(page.getByRole("gridcell", { name: "Brisas" })).toBeVisible();
  // El registro de intentos vive fuera de la grilla: se ve en cualquier
  // ancho (las columnas de la grilla se virtualizan en pantallas angostas).
  await page.getByText(/Intentos y alertas/).click();
  await expect(page.getByText("203.0.113.7")).toBeVisible();
});

test("Alta de dispositivo muestra el código y su QR sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/dispositivos");
  await page.getByRole("button", { name: "+ Nuevo" }).click();
  await page.getByPlaceholder("ej. Brisas - PC recepción").fill("PC nueva");
  await page.getByRole("button", { name: "Crear y generar código" }).click();

  await expect(page.getByText("K7QM-R4XT-2P")).toBeVisible();
  await expect(page.getByRole("img", { name: "Código QR de vinculación" })).toBeVisible();
  await expect(page.getByText(/Vence en \d+:\d{2}/)).toBeVisible();
});

test("Historial (AG Grid) y exportación a PDF sin violaciones de CSP", async ({ page }) => {
  await preparar(page);
  await page.goto("/historial");
  await expect(page.getByText("Contratista de prueba")).toBeVisible();
  // Paginación en el servidor: la grilla muestra el rango de la página y los
  // tres botones de exportación (Excel, CSV y PDF) como en escritorio.
  await expect(page.locator(".ag-paging-row-summary-content")).toHaveText("1 a 1 de 1");
  await expect(page.getByRole("button", { name: "Exportar a Excel" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Exportar a CSV" })).toBeVisible();  // El punto de mayor riesgo real: exportarAPdf() escribe HTML crudo con un
  // <style> inline en un iframe oculto (about:blank hereda el CSP del
  // documento que lo creó) -- si algo en la política bloquea ese estilo,
  // tiene que aparecer acá como violación real, no sólo en teoría. La
  // violación (si la hay) se dispara de forma síncrona al escribir el HTML,
  // antes de que `print()` haga nada -- no hace falta esperar un diálogo.
  await page.getByRole("button", { name: /exportar a pdf/i }).click();
  await page.waitForTimeout(500);
});
