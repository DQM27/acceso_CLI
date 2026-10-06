import { assertEquals } from "jsr:@std/assert@1";
import { type Cuenta, ORIGEN, planificar } from "./plan.ts";

const AHORA = Date.parse("2026-10-06T12:00:00Z");
const FUTURO = "2126-10-06T12:00:00Z";
const PASADO = "2026-01-01T00:00:00Z";

const cuenta = (id: string, email: string, extra: Partial<Cuenta> = {}): Cuenta => ({ id, email, ...extra });

Deno.test("crea la cuenta de un anfitrión activo que no la tiene, con el correo normalizado", () => {
  assertEquals(planificar([{ correo: "Ana@Empresa.example", activo: true }], [], [], AHORA), [
    { tipo: "crear", correo: "ana@empresa.example" },
  ]);
});

Deno.test("no crea cuentas para anfitriones inactivos ni duplica las existentes", () => {
  const acciones = planificar(
    [
      { correo: "baja@empresa.example", activo: false },
      { correo: "ana@empresa.example", activo: true },
    ],
    [],
    [cuenta("1", "ANA@empresa.example")],
    AHORA,
  );
  assertEquals(acciones, []);
});

Deno.test("bloquea al anfitrión dado de baja y marca que el bloqueo es suyo", () => {
  const acciones = planificar(
    [{ correo: "ana@empresa.example", activo: false }],
    [],
    [cuenta("1", "ana@empresa.example", { app_metadata: { provider: "email" } })],
    AHORA,
  );
  assertEquals(acciones, [
    { tipo: "bloquear", id: "1", app_metadata: { provider: "email", bloqueada_por: ORIGEN } },
  ]);
});

Deno.test("bloquea la cuenta creada acá cuyo anfitrión se borró de la tabla", () => {
  const acciones = planificar([], [], [cuenta("1", "ana@empresa.example", { app_metadata: { origen: ORIGEN } })], AHORA);
  assertEquals(acciones.map((a) => a.tipo), ["bloquear"]);
});

Deno.test("no vuelve a bloquear una cuenta ya bloqueada, pero sí una cuyo bloqueo venció", () => {
  const filas = [{ correo: "ana@empresa.example", activo: false }];
  assertEquals(planificar(filas, [], [cuenta("1", "ana@empresa.example", { banned_until: FUTURO })], AHORA), []);
  assertEquals(
    planificar(filas, [], [cuenta("1", "ana@empresa.example", { banned_until: PASADO })], AHORA).map((a) => a.tipo),
    ["bloquear"],
  );
});

Deno.test("reactivar desbloquea solo lo que bloqueó esta función", () => {
  const filas = [
    { correo: "ana@empresa.example", activo: true },
    { correo: "luis@empresa.example", activo: true },
  ];
  const acciones = planificar(
    filas,
    [],
    [
      cuenta("1", "ana@empresa.example", { banned_until: FUTURO, app_metadata: { bloqueada_por: ORIGEN } }),
      // Bloqueada a mano por otra razón: no se toca.
      cuenta("2", "luis@empresa.example", { banned_until: FUTURO, app_metadata: {} }),
    ],
    AHORA,
  );
  assertEquals(acciones, [{ tipo: "desbloquear", id: "1", app_metadata: { bloqueada_por: null } }]);
});

Deno.test("nunca bloquea administradores del panel ni operadores @brisas.local", () => {
  const acciones = planificar(
    [
      { correo: "jefe@gmail.example", activo: false },
      { correo: "110110110@brisas.local", activo: false },
    ],
    ["Jefe@gmail.example"],
    [cuenta("1", "jefe@gmail.example"), cuenta("2", "110110110@brisas.local")],
    AHORA,
  );
  assertEquals(acciones, []);
});

Deno.test("no toca cuentas ajenas a anfitriones", () => {
  const acciones = planificar(
    [],
    ["admin@gmail.example"],
    [cuenta("1", "admin@gmail.example"), cuenta("2", "220220220@brisas.local"), cuenta("3", "otra@gmail.example")],
    AHORA,
  );
  assertEquals(acciones, []);
});
