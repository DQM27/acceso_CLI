import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

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
  { id: uuid, nombre: "Brisas", direccion: "San José, Costa Rica" },
  {
    id: "00000000-0000-4000-8000-000000000002",
    nombre: "Cartago",
    direccion: "Cartago, Costa Rica",
  },
];

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const estado = window as unknown as { violacionesCsp: string[] };
    estado.violacionesCsp = [];
    document.addEventListener("securitypolicyviolation", (evento) =>
      estado.violacionesCsp.push(evento.violatedDirective),
    );
  });
});
test.afterEach(async ({ page }) => {
  const violaciones = await page.evaluate(
    () => (window as unknown as { violacionesCsp: string[] }).violacionesCsp,
  );
  expect(violaciones ?? []).toEqual([]);
});

async function preparar(
  page: Page,
  opciones: {
    autorizado?: boolean;
    falloGuardado?: boolean;
    cita?: boolean;
  } = {},
) {
  let guardados: Record<string, unknown>[] = [];
  let citas: unknown[] = opciones.cita
    ? [
        {
          id: uuid,
          anfitrion_correo: correo,
          motivo: "Reunión de coordinación",
          fecha_desde: "2099-09-10",
          fecha_hasta: "2099-09-11",
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
  await page.route(
    "https://xidaepyaljzkpbsxrqsm.supabase.co/**",
    async (ruta) => {
      const url = new URL(ruta.request().url());
      const responder = (datos: unknown, status = 200) =>
        ruta.fulfill({
          status,
          contentType: "application/json",
          body: JSON.stringify(datos),
        });
      if (url.pathname === "/auth/v1/user") return responder(usuario);
      if (url.pathname === "/auth/v1/logout") return responder({});
      if (url.pathname === "/rest/v1/anfitriones")
        return responder(
          opciones.autorizado === false
            ? null
            : { correo, nombre: "Daniel · Cuenta de prueba" },
        );
      if (url.pathname === "/rest/v1/sitios") return responder(sitios);
      if (url.pathname === "/rest/v1/rpc/crear_cita_anfitrion") {
        const datos = ruta.request().postDataJSON();
        guardados.push(datos);
        if (opciones.falloGuardado && guardados.length === 1)
          return responder({ code: "timeout", message: "fallo simulado" }, 503);
        return responder(datos.p_id);
      }
      if (url.pathname === "/rest/v1/citas") {
        if (ruta.request().method() === "PATCH") {
          citas = citas.map((cita) => ({
            ...(cita as object),
            estado: "CANCELADA",
          }));
          return responder({ id: uuid });
        }
        return responder(citas);
      }
      throw new Error(`Petición inesperada: ${url.pathname}`);
    },
  );
  return { guardados };
}

test("login con CSP real, sin desbordamiento en ambos temas", async ({
  page,
}, info) => {
  const errores: string[] = [];
  page.on("pageerror", (error) => errores.push(error.message));
  const respuesta = await page.goto("/");
  const csp = respuesta!.headers()["content-security-policy"];
  expect(csp).toContain("script-src 'self'");
  expect(csp).toContain("style-src 'self'");
  expect(csp).not.toContain("unsafe-");
  expect(respuesta!.headers()["referrer-policy"]).toBe("no-referrer");
  expect(respuesta!.headers()["x-frame-options"]).toBe("DENY");
  await expect(
    page.getByRole("button", { name: "Continuar con Google" }),
  ).toBeVisible();
  await page.screenshot({
    path: `test-results/login-${info.project.name}.png`,
    fullPage: true,
  });
  await page.getByRole("button", { name: /Cambiar a tema/ }).click();
  await page.screenshot({
    path: `test-results/login-tema-${info.project.name}.png`,
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  expect(errores).toEqual([]);
});

test("una cuenta sin autorización no ve la agenda", async ({ page }) => {
  await preparar(page, { autorizado: false });
  await page.goto("/citas");
  await expect(page.getByRole("alert")).toContainText("no está autorizada");
  await expect(
    page.getByRole("heading", { name: "Mis citas", exact: true }),
  ).toHaveCount(0);
});

test("grupo con dos sitios, validación y reintento idempotente", async ({
  page,
}, info) => {
  const { guardados } = await preparar(page, { falloGuardado: true });
  await page.goto("/nueva");
  await expect(
    page.getByRole("heading", { name: "Nueva cita", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Revisar cita" }).click();
  await expect(page.getByText("Seleccioná al menos un sitio.")).toBeVisible();
  await page.getByRole("checkbox", { name: /Brisas/ }).check();
  await page.getByRole("checkbox", { name: /Cartago/ }).check();
  await page.getByLabel("Nombre completo").fill("Persona de prueba Uno");
  await page.getByLabel("Cédula o documento").fill("DOC-123");
  await page.getByRole("button", { name: "Agregar visitante" }).click();
  await page.getByLabel("Nombre completo").nth(1).fill("Persona de prueba Dos");
  await page.getByLabel("Cédula o documento").nth(1).fill("DOC123");
  await page.getByRole("button", { name: "Revisar cita" }).click();
  await expect(
    page.getByText("Este documento ya está en la lista."),
  ).toBeVisible();
  await page.getByLabel("Cédula o documento").nth(1).fill("DOC456");
  await page.screenshot({
    path: `test-results/nueva-${info.project.name}.png`,
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Revisar cita" }).click();
  await expect(
    page.getByRole("heading", { name: "Revisá tu cita" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Confirmar y agendar" }).click();
  await expect(page.getByRole("alert")).toContainText("no está confirmado");
  await page.getByRole("button", { name: "Reintentar guardado" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Cita agendada" }),
  ).toBeVisible();
  expect(guardados).toHaveLength(2);
  expect(guardados[0]).toEqual(guardados[1]);
  expect(guardados[0]?.p_sitios).toHaveLength(2);
  expect(guardados[0]?.p_visitantes).toHaveLength(2);
});

test("detalles, cancelación confirmada y modal por teclado", async ({
  page,
}, info) => {
  await preparar(page, { cita: true });
  await page.goto("/citas");
  await expect(page.getByText("Reunión de coordinación")).toBeVisible();
  await page.screenshot({
    path: `test-results/agenda-${info.project.name}.png`,
    fullPage: true,
  });
  await page.getByRole("button", { name: "Ver detalles" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("dialog")).toContainText("DOC123");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Ver detalles" }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "Cancelar cita", exact: true })
    .click();
  await page.getByRole("button", { name: "Sí, cancelar cita" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "se canceló correctamente" }),
  ).toBeVisible();
  await expect(page.locator(".estado-cancelada")).toBeVisible();
});

test("confirma abandonar el formulario y no pierde datos al quedarse", async ({
  page,
}) => {
  await preparar(page);
  await page.goto("/nueva");
  await page.getByLabel("Nombre completo").fill("Persona de prueba");
  await page.locator(".volver").click();
  await expect(page.getByRole("dialog")).toContainText("cambios sin guardar");
  await page.getByRole("button", { name: "Seguir aquí" }).click();
  await expect(page.getByLabel("Nombre completo")).toHaveValue(
    "Persona de prueba",
  );
});

test("la pérdida de conexión conserva el formulario y bloquea el envío", async ({
  page,
  context,
}) => {
  await preparar(page);
  await page.goto("/nueva");
  await page.getByLabel("Nombre completo").fill("Persona de prueba");
  await context.setOffline(true);
  await expect(page.getByRole("alert")).toContainText("sin conexión");
  await expect(
    page.getByRole("button", { name: "Revisar cita" }),
  ).toBeDisabled();
  await expect(page.getByLabel("Nombre completo")).toHaveValue(
    "Persona de prueba",
  );
  await context.setOffline(false);
  await expect(
    page.getByRole("button", { name: "Revisar cita" }),
  ).toBeEnabled();
  await expect(page.getByLabel("Nombre completo")).toHaveValue(
    "Persona de prueba",
  );
});
