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
  { id: uuid, nombre: "Brisas" },
  { id: "00000000-0000-4000-8000-000000000002", nombre: "Cartago" },
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
  const guardados: Record<string, unknown>[] = [];
  let citas: unknown[] = opciones.cita
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

/** Abre el combobox de "Sitios de la visita" (componentes/SelectorSitios.tsx),
 * tilda cada nombre y lo cierra -- reemplaza el click directo sobre los
 * chips que había antes de la Fase de rediseño del selector. */
async function elegirSitios(page: Page, nombres: string[]) {
  const selector = page.locator(".selector-sitios");
  await selector.getByRole("button").click();
  for (const nombre of nombres) {
    await selector.getByText(nombre, { exact: true }).click();
  }
  await page.keyboard.press("Escape");
}

test("login con CSP real, sin desbordamiento en ambos temas", async ({
  page,
}, info) => {
  const errores: string[] = [];
  page.on("pageerror", (error) => errores.push(error.message));
  const respuesta = await page.goto("/");
  if (!respuesta) throw new Error("La navegación a / no devolvió respuesta");
  const csp = respuesta.headers()["content-security-policy"];
  expect(csp).toContain("script-src 'self'");
  expect(csp).toContain("style-src 'self'");
  expect(csp).not.toContain("unsafe-");
  expect(respuesta.headers()["referrer-policy"]).toBe("no-referrer");
  expect(respuesta.headers()["x-frame-options"]).toBe("DENY");
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

test("Activity conserva el mes del calendario al ir y volver entre pasos", async ({
  page,
}) => {
  const errores: string[] = [];
  page.on("pageerror", (error) => errores.push(error.message));
  await preparar(page);
  await page.goto("/nueva");
  await expect(page.getByText("septiembre de 2026")).toBeVisible();
  await page.getByRole("button", { name: "Mes siguiente" }).click();
  await expect(page.getByText("octubre de 2026")).toBeVisible();
  await elegirSitios(page, ["Brisas"]);
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(page.getByLabel("Nombre completo")).toBeVisible();
  await page.getByRole("button", { name: "Atrás" }).click();
  // El calendario (SelectorFechas.tsx) sigue montado -- oculto por
  // <Activity>, no destruido -- así que conserva el mes al que se había
  // navegado en vez de volver al mes de "hoy".
  await expect(page.getByText("octubre de 2026")).toBeVisible();
  expect(errores).toEqual([]);
});

test("las fechas se completan 100% por teclado, sin tocar el calendario", async ({
  page,
}) => {
  const { guardados } = await preparar(page);
  await page.goto("/nueva");
  // El stepper (componentes/PasoWizard.tsx) debe verse en TODOS los
  // breakpoints -- este mismo test corre también bajo el proyecto "movil"
  // (viewport angosto), a diferencia del viejo .indicador-paso que se
  // ocultaba por completo ahí.
  await expect(
    page.getByRole("navigation", { name: "Progreso de la cita" }),
  ).toBeVisible();
  await expect(
    page.getByText("Paso 1 de 3: ¿Cuándo y dónde?"),
  ).toBeAttached();
  await elegirSitios(page, ["Brisas"]);
  // Día/Mes/Año como 3 campos de texto es la vía de teclado real -- nunca
  // se hace click ni drag sobre el calendario en este test.
  const desde = page.getByRole("group", { name: "Desde" });
  await desde.getByLabel("Día").fill("10");
  await desde.getByLabel("Mes").fill("09");
  await desde.getByLabel("Año").fill("2099");
  const hasta = page.getByRole("group", { name: "Hasta" });
  await hasta.getByLabel("Día").fill("12");
  await hasta.getByLabel("Mes").fill("09");
  await hasta.getByLabel("Año").fill("2099");
  await page.getByRole("button", { name: "Continuar" }).click();
  await page.getByLabel("Nombre completo").fill("Persona de prueba");
  await page.getByLabel("Cédula o documento").fill("DOC123");
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(
    page.getByRole("heading", { name: "Revisá tu cita" }),
  ).toBeVisible();
  await expect(page.getByText("10 sept 2099 — 12 sept 2099")).toBeVisible();
  await page.getByRole("button", { name: "Confirmar y agendar" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Cita agendada" }),
  ).toBeVisible();
  expect(guardados[0]).toMatchObject({
    p_fecha_desde: "2099-09-10",
    p_fecha_hasta: "2099-09-12",
  });
});

test("grupo grande de visitantes: se colapsan, se pueden reabrir y la validación de duplicados sigue funcionando", async ({
  page,
}) => {
  await preparar(page);
  await page.goto("/nueva");
  await elegirSitios(page, ["Brisas"]);
  await page.getByRole("button", { name: "Continuar" }).click();
  // 1 visitante ya existe por defecto -- se agregan 5 más (6 en total,
  // por encima del umbral de colapso).
  for (let i = 0; i < 5; i++) {
    await page.getByRole("button", { name: "Agregar visitante" }).click();
  }
  for (let i = 0; i < 6; i++) {
    await page.getByLabel("Nombre completo").nth(i).fill(`Persona ${i + 1}`);
    await page.getByLabel("Cédula o documento").nth(i).fill(`DOC00${i + 1}`);
  }
  // Con 6 visitantes, uno del medio (ni el primero en pantalla ni el
  // último agregado) queda colapsado por defecto -- su input no está
  // visible aunque siga en el DOM.
  await expect(page.getByLabel("Nombre completo").nth(2)).toBeHidden();
  // Reabrirlo a mano (click en el <summary>) sigue funcionando.
  await page.getByText("Persona 3 · DOC003").click();
  await expect(page.getByLabel("Nombre completo").nth(2)).toBeVisible();
  await page.getByLabel("Nombre completo").nth(2).fill("Persona 3 editada");
  // Un duplicado entre dos visitantes cualesquiera se sigue detectando
  // igual, sin importar cuántos haya ni cuáles estén colapsados -- el
  // último (índice 5) queda abierto por defecto por ser el recién
  // agregado, sin necesidad de reabrirlo a mano primero.
  await page.getByLabel("Cédula o documento").nth(5).fill("DOC003");
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(
    page.getByText("Este documento ya está en la lista."),
  ).toBeVisible();
  await page.getByLabel("Cédula o documento").nth(5).fill("DOC006");
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(
    page.getByRole("heading", { name: "Revisá tu cita" }),
  ).toBeVisible();
  // El paso "Visitantes" sigue montado (oculto por <Activity>), así que su
  // propio resumen colapsado también contiene este texto -- se acota a la
  // sección de revisión, igual que ya se hizo para "hora_estimada".
  await expect(
    page.locator(".bloque-revision").getByText("Persona 3 editada"),
  ).toBeVisible();
  await expect(page.getByText("Visitantes (6)")).toBeVisible();
});

test("grupo con dos sitios, validación y reintento idempotente", async ({
  page,
}, info) => {
  const { guardados } = await preparar(page, { falloGuardado: true });
  await page.goto("/nueva");
  await expect(
    page.getByRole("heading", { name: "Nueva cita", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(page.getByText("Seleccioná al menos un sitio.")).toBeVisible();
  await elegirSitios(page, ["Brisas", "Cartago"]);
  await page.getByRole("button", { name: "Continuar" }).click();
  await page.getByLabel("Nombre completo").fill("Persona de prueba Uno");
  await page.getByLabel("Cédula o documento").fill("DOC-123");
  await page.getByRole("button", { name: "Agregar visitante" }).click();
  await page.getByLabel("Nombre completo").nth(1).fill("Persona de prueba Dos");
  await page.getByLabel("Cédula o documento").nth(1).fill("DOC123");
  await page.getByRole("button", { name: "Continuar" }).click();
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
  await page.getByRole("button", { name: "Continuar" }).click();
  await expect(
    page.getByRole("heading", { name: "Revisá tu cita" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Confirmar y agendar" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "No pudimos confirmar si tu cita quedó guardada",
  );
  await page.getByRole("button", { name: "Reintentar guardado" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Cita agendada" }),
  ).toBeVisible();
  expect(guardados).toHaveLength(2);
  expect(guardados[0]).toEqual(guardados[1]);
  expect(guardados[0]?.p_sitios).toHaveLength(2);
  expect(guardados[0]?.p_visitantes).toHaveLength(2);
});

test("hora estimada es opcional, viaja a la RPC y se ve en Mis Citas", async ({
  page,
}) => {
  const { guardados } = await preparar(page);
  await page.goto("/nueva");
  await elegirSitios(page, ["Brisas"]);
  await page.getByLabel(/Hora aproximada de llegada/).fill("14:30");
  await page.getByRole("button", { name: "Continuar" }).click();
  await page.getByLabel("Nombre completo").fill("Persona de prueba");
  await page.getByLabel("Cédula o documento").fill("DOC123");
  await page.getByRole("button", { name: "Continuar" }).click();
  // Aparece dos veces a propósito (resumen lateral + detalle principal,
  // mismo patrón que "Fechas"/"Sitios") -- se acota a una sola zona.
  await expect(
    page.locator(".bloque-revision").getByText("2:30 p. m."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Confirmar y agendar" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Cita agendada" }),
  ).toBeVisible();
  expect(guardados[0]?.p_hora_estimada).toBe("14:30");
});

test("detalles, cancelación confirmada y modal por teclado", async ({
  page,
}, info) => {
  await preparar(page, { cita: true });
  await page.goto("/citas");
  await expect(page.getByText("Reunión de coordinación")).toBeVisible();
  await expect(page.getByText("10:00 a. m.")).toBeVisible();
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
  await page.getByLabel(/Motivo de la visita/).fill("Reunión de prueba");
  await page.locator(".volver").click();
  await expect(page.getByRole("dialog")).toContainText("cambios sin guardar");
  await page.getByRole("button", { name: "Seguir aquí" }).click();
  await expect(page.getByLabel(/Motivo de la visita/)).toHaveValue(
    "Reunión de prueba",
  );
});

test("la pérdida de conexión conserva el formulario y bloquea el envío", async ({
  page,
  context,
}) => {
  await preparar(page);
  await page.goto("/nueva");
  await page.getByLabel(/Motivo de la visita/).fill("Reunión de prueba");
  await context.setOffline(true);
  await expect(page.getByRole("alert")).toContainText("sin conexión");
  await expect(page.getByRole("button", { name: "Continuar" })).toBeDisabled();
  await expect(page.getByLabel(/Motivo de la visita/)).toHaveValue(
    "Reunión de prueba",
  );
  await context.setOffline(false);
  await expect(page.getByRole("button", { name: "Continuar" })).toBeEnabled();
  await expect(page.getByLabel(/Motivo de la visita/)).toHaveValue(
    "Reunión de prueba",
  );
});
