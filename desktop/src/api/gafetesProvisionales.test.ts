import { describe, expect, it } from "vitest";
import {
  claveFilaGafeteProvisionalActiva,
  filaGafeteProvisionalDesdeLocal,
  filaGafeteProvisionalDesdeRemoto,
} from "./gafetesProvisionales";
import type {
  PrestamoGafeteProvisionalActivoResumen,
  PrestamoGafeteProvisionalRemoto,
} from "./gafetesProvisionales";

// Mismo criterio que activos.test.ts (filaDesdeLocal/filaDesdeRemoto) --
// esta capa de fusión local+remoto no tenía ningún test, a pesar de ser
// exactamente el tipo de mapeo donde un campo nuevo (usuario_entrega_nombre,
// 2026-09-21) se puede olvidar en silencio sin que ningún test de Rust lo
// note -- el núcleo nunca ve este archivo.
describe("filaGafeteProvisionalDesdeLocal / filaGafeteProvisionalDesdeRemoto", () => {
  it("una fila local conserva sus datos y queda marcada origen: local", () => {
    const item: PrestamoGafeteProvisionalActivoResumen = {
      id: 5,
      encargado_nombre: "Michael Araya Retana",
      encargado_codigo_empleado: "5040017",
      gafete_numero: 12,
      fecha_hora_entrega: "2027-03-08T12:00:00Z",
      usuario_entrega_nombre: "Operador",
    };

    const resultado = filaGafeteProvisionalDesdeLocal(item);

    expect(resultado).toMatchObject({
      origen: "local",
      id: 5,
      encargado_nombre: "Michael Araya Retana",
      encargado_codigo_empleado: "5040017",
      gafete_numero: 12,
      usuario_entrega_nombre: "Operador",
    });
  });

  it("una fila remota no trae id local, pero sí el resto de los datos que ya manda la nube", () => {
    const remoto: PrestamoGafeteProvisionalRemoto = {
      uuid: "uuid-remoto",
      encargado_nombre: "Ramon Rodriguez",
      encargado_codigo_empleado: "77851",
      gafete_numero: 8,
      hora_entrega: "2027-03-08T08:00:00Z",
      usuario_entrega_nombre: "Op PC",
    };

    const resultado = filaGafeteProvisionalDesdeRemoto(remoto);

    expect(resultado).toMatchObject({
      origen: "remoto",
      uuid_remoto: "uuid-remoto",
      id: null,
      encargado_nombre: "Ramon Rodriguez",
      encargado_codigo_empleado: "77851",
      gafete_numero: 8,
      fecha_hora_entrega: "2027-03-08T08:00:00Z",
      usuario_entrega_nombre: "Op PC",
    });
  });
});

describe("claveFilaGafeteProvisionalActiva", () => {
  it("una fila local usa su id", () => {
    const fila = filaGafeteProvisionalDesdeLocal({
      id: 9,
      encargado_nombre: "Michael Araya Retana",
      encargado_codigo_empleado: "5040017",
      gafete_numero: 1,
      fecha_hora_entrega: "2027-03-08T12:00:00Z",
      usuario_entrega_nombre: "Operador",
    });

    expect(claveFilaGafeteProvisionalActiva(fila)).toBe("local-9");
  });

  it("una fila remota usa su uuid, no `id` (que es null)", () => {
    const fila = filaGafeteProvisionalDesdeRemoto({
      uuid: "uuid-remoto-x",
      encargado_nombre: "Ramon Rodriguez",
      encargado_codigo_empleado: "77851",
      gafete_numero: 2,
      hora_entrega: "2027-03-08T08:00:00Z",
      usuario_entrega_nombre: "Op PC",
    });

    expect(claveFilaGafeteProvisionalActiva(fila)).toBe("remoto-uuid-remoto-x");
  });
});
