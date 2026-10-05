import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

// "Hoy" fijo: mediodía del lunes 5 de octubre de 2026 en Costa Rica, lejos
// de cualquier borde de día. Sin esto las pruebas dependen de la fecha real.
const AHORA = new Date("2026-10-05T12:00:00-06:00");
const HOY = "2026-10-05";

const id = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
const correo = "anfitrion@example.invalid";
const usuario = {
  id: id(1),
  aud: "authenticated",
  role: "authenticated",
  email: correo,
  app_metadata: { provider: "google", providers: ["google"] },
  user_metadata: {},
  created_at: "2026-09-09T12:00:00Z",
};
const sitios = [
  { id: id(11), nombre: "Brisas" },
  { id: id(12), nombre: "Cartago" },
];

type Cita = Record<string, unknown> & { id: string; estado: string; fecha_hasta: string };

function cita(n: number, cambios: Partial<Cita> = {}): Cita {
  return {
    id: id(n),
    anfitrion_correo: correo,
    motivo: null,
    fecha_desde: HOY,
    fecha_hasta: HOY,
    hora_estimada: null,
    estado: "VIGENTE",
    created_at: "2026-10-01T15:00:00Z",
    cita_sitios: [{ sitio_id: sitios[0].id, sitios: sitios[0] }],
    cita_visitantes: [
      { id: id(n * 100 + 1), nombre: "Ana Mora", cedula: "112340567", empresa: "ACME", placa_vehiculo: null },
    ],
    ...cambios,
  };
}

const deHoy = cita(21, {
  motivo: "Auditoría de seguridad",
  hora_estimada: "09:00:00",
  cita_visitantes: [
    { id: id(2101), nombre: "Ana Mora", cedula: "112340567", empresa: "ACME", placa_vehiculo: null },
    { id: id(2102), nombre: "Luis Rojas", cedula: "204560789", empresa: "ACME", placa_vehiculo: "BCD123" },
  ],
});
const proxima = cita(22, { motivo: "Mantenimiento de aires", fecha_desde: "2026-10-07", fecha_hasta: "2026-10-08" });
const cancelada = cita(23, { motivo: "Reunión suspendida", estado: "CANCELADA" });

test.beforeEach(async ({ page }) => {
  await page.clock.setFixedTime(AHORA);
  await page.addInitScript(() => {
    const estado = window as unknown as { violacionesCsp: string[] };
    estado.violacionesCsp = [];
    document.addEventListener("securitypolicyviolation", (evento) => estado.violacionesCsp.push(evento.violatedDirective));
  });
});
test.afterEach(async ({ page }) => {
  const violaciones = await page.evaluate(() => (window as unknown as { violacionesCsp: string[] }).violacionesCsp);
  expect(violaciones ?? []).toEqual([]);
});

async function sinDesborde(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
}

/** Sesión de prueba y una nube falsa en memoria (citas y RPCs). */
async function preparar(
  page: Page,
  opciones: { autorizado?: boolean; falloGuardado?: boolean; citas?: Cita[]; llegadas?: unknown[] } = {},
) {
  const llamadas: { rpc: string; datos: Record<string, unknown> }[] = [];
  let citas = [...(opciones.citas ?? [])];
  let fallosPendientes = opciones.falloGuardado ? 1 : 0;
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
  await page.route("https://xidaepyaljzkpbsxrqsm.supabase.co/**", async (ruta) => {
    const peticion = ruta.request();
    const url = new URL(peticion.url());
    const responder = (datos: unknown, status = 200) =>
      ruta.fulfill({ status, contentType: "application/json", body: JSON.stringify(datos) });
    if (url.pathname === "/auth/v1/user") return responder(usuario);
    if (url.pathname === "/auth/v1/logout") return responder({});
    if (url.pathname === "/rest/v1/anfitriones")
      return responder(opciones.autorizado === false ? null : { correo, nombre: "Daniel Quintana" });
    if (url.pathname === "/rest/v1/sitios") return responder(sitios);
    if (url.pathname === "/rest/v1/citas") {
      const porId = url.searchParams.get("id")?.replace("eq.", "");
      if (porId) return responder(citas.filter((c) => c.id === porId));
      if (url.searchParams.get("or")) return responder(citas.filter((c) => c.estado === "CANCELADA" || c.fecha_hasta < HOY));
      return responder(citas.filter((c) => c.estado === "VIGENTE" && c.fecha_hasta >= HOY));
    }
    if (url.pathname.startsWith("/rest/v1/rpc/")) {
      const rpc = url.pathname.slice("/rest/v1/rpc/".length);
      const datos = peticion.postDataJSON() as Record<string, unknown>;
      llamadas.push({ rpc, datos });
      if (rpc === "visitantes_anteriores")
        return responder([
          { cedula: "305670891", nombre: "Carla Vargas", empresa: "Limpiezas del Sur", placa_vehiculo: null, ultima_vez: "2026-09-20" },
        ]);
      if (rpc === "estado_visitantes_de_mis_citas") return responder(opciones.llegadas ?? []);
      if (rpc === "crear_cita_anfitrion" || rpc === "editar_cita_anfitrion") {
        if (fallosPendientes > 0) {
          fallosPendientes--;
          return responder({ code: "timeout", message: "fallo simulado" }, 503);
        }
        const nueva = String(rpc === "crear_cita_anfitrion" ? datos.p_id : datos.p_nuevo_id);
        if (rpc === "editar_cita_anfitrion")
          citas = citas.map((c) => (c.id === datos.p_id ? { ...c, estado: "CANCELADA" } : c));
        citas.push(
          cita(99, {
            id: nueva,
            motivo: (datos.p_motivo as string | null) ?? null,
            fecha_desde: datos.p_fecha_desde as string,
            fecha_hasta: datos.p_fecha_hasta as string,
          }),
        );
        return responder(nueva);
      }
      if (rpc === "cancelar_cita_anfitrion") {
        citas = citas.map((c) => (c.id === datos.p_id ? { ...c, estado: "CANCELADA" } : c));
        return responder(null);
      }
    }
    throw new Error(`Petición inesperada: ${peticion.method()} ${url.pathname}`);
  });
  return { llamadas: (rpc: string) => llamadas.filter((l) => l.rpc === rpc).map((l) => l.datos) };
}

test("login con CSP real, sin desbordamiento en ambos temas", async ({ page }, info) => {
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
  await expect(page.getByRole("button", { name: "Continuar con Google" })).toBeVisible();
  await page.screenshot({ path: `test-results/login-${info.project.name}.png`, fullPage: true });
  await page.getByRole("button", { name: /Cambiar a tema/ }).click();
  await page.screenshot({ path: `test-results/login-tema-${info.project.name}.png`, fullPage: true });
  await sinDesborde(page);
  expect(errores).toEqual([]);
});

test("una cuenta sin autorización no ve la agenda", async ({ page }) => {
  await preparar(page, { autorizado: false });
  await page.goto("/visitas");
  await expect(page.getByRole("alert")).toContainText("no está autorizada");
  await expect(page.getByRole("heading", { name: "Mis visitas" })).toHaveCount(0);
});

test("Mis visitas: hoy con quién llegó, próximas e historial", async ({ page }, info) => {
  const { llamadas } = await preparar(page, {
    citas: [deHoy, proxima, cancelada],
    llegadas: [
      {
        cita_visitante_id: id(2101),
        sitio_nombre: "Brisas",
        hora_entrada: "2026-10-05T15:12:00Z",
        hora_salida: null,
        gafete_numero: 7,
      },
    ],
  });
  // Las rutas viejas llevan a la pantalla nueva.
  await page.goto("/citas");
  await expect(page).toHaveURL(/\/visitas$/);
  await expect(page.getByRole("heading", { name: "Mis visitas" })).toBeVisible();
  const hoy = page.getByRole("region", { name: "Hoy" });
  await expect(hoy.getByText("Auditoría de seguridad")).toBeVisible();
  await expect(hoy.getByText("Llegó 9:12 · gafete 7")).toBeVisible();
  await expect(hoy.getByText("Sin llegar")).toBeVisible();
  await expect(hoy.getByText("9:00")).toBeVisible();
  await expect(page.getByRole("region", { name: "Próximas" }).getByText("Mantenimiento de aires")).toBeVisible();
  await expect(page.getByText("Reunión suspendida")).toHaveCount(0);
  expect(llamadas("estado_visitantes_de_mis_citas")[0]).toEqual({ p_citas: [deHoy.id] });
  await page.screenshot({ path: `test-results/mis-visitas-${info.project.name}.png`, fullPage: true });
  await sinDesborde(page);

  await page.getByRole("link", { name: "Ver historial" }).click();
  await expect(page.getByText("Reunión suspendida")).toBeVisible();
  await expect(page.getByText("Auditoría de seguridad")).toHaveCount(0);
});

test("agendar: valida, dos lugares, reintento idempotente y aviso", async ({ page }, info) => {
  const { llamadas } = await preparar(page, { falloGuardado: true });
  await page.goto("/visitas");
  await page.getByRole("link", { name: "Agendar" }).click();
  await expect(page).toHaveURL(/\/agendar$/);

  await page.getByRole("button", { name: "Agendar", exact: true }).click();
  await expect(page.getByRole("alert").first()).toContainText("Revise los datos marcados.");
  await expect(page.getByText("Ingrese el nombre completo.")).toBeVisible();
  await expect(page.getByText("Elija al menos un lugar.")).toBeVisible();

  await page.getByLabel(/^Nombre/).fill("Ana Mora");
  await page.getByLabel("Cédula o documento").fill("1-1234-0567");
  await page.getByRole("button", { name: "Agregar persona" }).click();
  await page.getByLabel(/^Nombre/).nth(1).fill("Luis Rojas");
  await page.getByLabel("Cédula o documento").nth(1).fill("112340567");
  await page.getByRole("button", { name: "Mañana" }).click();
  await page.getByLabel("Hora estimada").fill("14:30");
  await page.getByRole("button", { name: "Brisas" }).click();
  await page.getByRole("button", { name: "Cartago" }).click();
  await page.getByLabel("Motivo").fill("Revisión de equipos");
  await page.getByRole("button", { name: "Agendar", exact: true }).click();
  await expect(page.getByText("Esta persona ya está en la lista.")).toBeVisible();
  await page.getByLabel("Cédula o documento").nth(1).fill("204560789");
  await expect(page.getByText("2 personas · mar 6 oct · 14:30")).toBeVisible();
  await page.screenshot({ path: `test-results/agendar-${info.project.name}.png`, fullPage: true });
  await sinDesborde(page);

  // La primera respuesta se pierde: se avisa y el reintento manda lo mismo.
  await page.getByRole("button", { name: "Agendar", exact: true }).click();
  await expect(page.getByRole("alert").first()).toContainText("No se pudo completar la solicitud");
  await page.getByRole("button", { name: "Agendar", exact: true }).click();
  await expect(page.getByText("Visita agendada: mar 6 oct")).toBeVisible();
  await expect(page).toHaveURL(/\/visitas$/);
  const guardados = llamadas("crear_cita_anfitrion");
  expect(guardados).toHaveLength(2);
  expect(guardados[0]).toEqual(guardados[1]);
  expect(guardados[0]).toMatchObject({
    p_fecha_desde: "2026-10-06",
    p_fecha_hasta: "2026-10-06",
    p_hora_estimada: "14:30",
    p_motivo: "Revisión de equipos",
    p_sitios: [sitios[0].id, sitios[1].id],
    p_visitantes: [
      expect.objectContaining({ nombre: "Ana Mora", cedula: "112340567" }),
      expect.objectContaining({ nombre: "Luis Rojas", cedula: "204560789" }),
    ],
  });
  await expect(page.getByRole("region", { name: "Próximas" }).getByText("Revisión de equipos")).toBeVisible();
});

test("agendar a alguien que ya vino, con un toque", async ({ page }) => {
  const { llamadas } = await preparar(page);
  await page.goto("/agendar");
  await page.getByRole("button", { name: "Carla Vargas" }).click();
  await expect(page.getByLabel(/^Nombre/)).toHaveValue("Carla Vargas");
  await expect(page.getByLabel("Empresa")).toHaveValue("Limpiezas del Sur");
  await expect(page.getByLabel(/^Nombre/)).toHaveCount(1);
  await page.getByRole("button", { name: "Brisas" }).click();
  await page.getByRole("button", { name: "Agendar", exact: true }).click();
  await expect(page.getByText(/Visita agendada/)).toBeVisible();
  expect(llamadas("crear_cita_anfitrion")[0]).toMatchObject({
    p_fecha_desde: HOY,
    p_hora_estimada: null,
    p_visitantes: [expect.objectContaining({ cedula: "305670891" })],
  });
});

test("detalle: cancelar con confirmación y diálogo por teclado", async ({ page }, info) => {
  const { llamadas } = await preparar(page, { citas: [deHoy] });
  await page.goto(`/visitas/${deHoy.id}`);
  await expect(page.getByRole("heading", { name: "Auditoría de seguridad" })).toBeVisible();
  await expect(page.getByText("204560789 · ACME · placa BCD123")).toBeVisible();
  await page.screenshot({ path: `test-results/detalle-${info.project.name}.png`, fullPage: true });
  await sinDesborde(page);

  const cancelar = page.getByRole("button", { name: "Cancelar visita" });
  await cancelar.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(cancelar).toBeFocused();

  await cancelar.click();
  await page.getByRole("button", { name: "Sí, cancelar" }).click();
  await expect(page.getByText("Visita cancelada.")).toBeVisible();
  await expect(page).toHaveURL(/\/visitas$/);
  expect(llamadas("cancelar_cita_anfitrion")).toEqual([{ p_id: deHoy.id }]);
  await expect(page.getByText("No tiene visitas para hoy.")).toBeVisible();
});

test("editar: cancela la vieja y abre la nueva", async ({ page }) => {
  const { llamadas } = await preparar(page, { citas: [proxima] });
  await page.goto(`/visitas/${proxima.id}`);
  await page.getByRole("link", { name: "Editar" }).click();
  await expect(page.getByLabel("Motivo")).toHaveValue("Mantenimiento de aires");
  await expect(page.getByRole("button", { name: "Varios días" })).toHaveAttribute("aria-pressed", "true");
  await page.getByLabel("Motivo").fill("Mantenimiento de aires (segunda visita)");
  await page.getByRole("button", { name: "Guardar cambios" }).click();
  await expect(page.getByText("Visita actualizada.")).toBeVisible();
  const [edicion] = llamadas("editar_cita_anfitrion");
  expect(edicion).toMatchObject({
    p_id: proxima.id,
    p_fecha_desde: "2026-10-07",
    p_fecha_hasta: "2026-10-08",
    p_motivo: "Mantenimiento de aires (segunda visita)",
  });
  expect(edicion?.p_nuevo_id).not.toBe(proxima.id);
  await expect(page).toHaveURL(new RegExp(`/visitas/${String(edicion?.p_nuevo_id)}$`));
  await expect(page.getByRole("heading", { name: "Mantenimiento de aires (segunda visita)" })).toBeVisible();
});

test("si alguien ya entró no se ofrece editar", async ({ page }) => {
  await preparar(page, {
    citas: [deHoy],
    llegadas: [
      {
        cita_visitante_id: id(2101),
        sitio_nombre: "Brisas",
        hora_entrada: "2026-10-05T15:12:00Z",
        hora_salida: "2026-10-05T17:40:00Z",
        gafete_numero: 7,
      },
    ],
  });
  await page.goto(`/visitas/${deHoy.id}`);
  await expect(page.getByText("Salió 11:40")).toBeVisible();
  await expect(page.getByText(/ya entró: ya no se puede editar/)).toBeVisible();
  await expect(page.getByRole("link", { name: "Editar" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Duplicar" })).toBeVisible();
});

test("confirma salir sin guardar y no pierde lo escrito al quedarse", async ({ page }) => {
  await preparar(page);
  await page.goto("/visitas");
  await page.getByRole("link", { name: "Agendar" }).click();
  await page.getByLabel("Motivo").fill("Reunión de prueba");
  await page.getByRole("link", { name: "Volver" }).click();
  await expect(page.getByRole("dialog")).toContainText("¿Salir sin guardar?");
  await page.getByRole("button", { name: "Seguir editando" }).click();
  await expect(page.getByLabel("Motivo")).toHaveValue("Reunión de prueba");
  await page.getByRole("link", { name: "Volver" }).click();
  await page.getByRole("button", { name: "Salir" }).click();
  await expect(page).toHaveURL(/\/visitas$/);
});

test("sin conexión conserva el formulario y bloquea el envío", async ({ page, context }) => {
  await preparar(page);
  await page.goto("/agendar");
  await page.getByLabel("Motivo").fill("Reunión de prueba");
  await context.setOffline(true);
  await expect(page.getByRole("alert").filter({ hasText: "Sin conexión" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Agendar", exact: true })).toBeDisabled();
  await context.setOffline(false);
  await expect(page.getByRole("button", { name: "Agendar", exact: true })).toBeEnabled();
  await expect(page.getByLabel("Motivo")).toHaveValue("Reunión de prueba");
});
