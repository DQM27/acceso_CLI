import { describe, expect, it } from "vitest";
import { colorEstadoAcceso, columnasPara, textoEstadoAcceso } from "./Contratistas.logica";

function campos(actorRol: Parameters<typeof columnasPara>[0]): (string | undefined)[] {
  return columnasPara(actorRol).map((c) => c.field);
}

describe("columnasPara (Contratistas)", () => {
  it("Operador no ve la columna 'Acceso' -- se delega al panel web", () => {
    expect(campos("Operador")).not.toContain("tiene_acceso");
  });

  it("Administrador y Root sí ven 'Acceso'", () => {
    expect(campos("Administrador")).toContain("tiene_acceso");
    expect(campos("Root")).toContain("tiene_acceso");
  });

  it("las columnas base siempre están, sin importar el rol", () => {
    for (const rol of ["Operador", "Administrador", "Root"] as const) {
      expect(campos(rol)).toEqual(
        expect.arrayContaining([
          "cedula",
          "nombre",
          "empresa_nombre",
          "tipo_ingreso",
          "fecha_vencimiento_praind",
          "es_personal_ruta",
          "tiene_ingreso_activo",
        ]),
      );
    }
  });
});

describe("estado de acceso (mismas reglas y textos que el panel)", () => {
  const fila = (estado_acceso: Parameters<typeof colorEstadoAcceso>[0], dias: number | null = null) => ({
    estado_acceso: estado_acceso ?? null,
    dias_para_vencer_praind: dias,
  });

  it("dice cuántos días faltan o pasaron de la PRAIND", () => {
    expect(textoEstadoAcceso(fila("PERMITIDO_CON_ADVERTENCIA", 5))).toBe("Puede entrar (PRAIND vence en 5 días)");
    expect(textoEstadoAcceso(fila("PERMITIDO_CON_ADVERTENCIA", 1))).toBe("Puede entrar (PRAIND vence en 1 día)");
    expect(textoEstadoAcceso(fila("PERMITIDO_CON_ADVERTENCIA", 0))).toBe("Puede entrar (PRAIND vence hoy)");
    expect(textoEstadoAcceso(fila("PRAIND_VENCIDO", -3))).toBe("PRAIND vencida hace 3 días");
  });

  it("los demás estados con el texto del panel", () => {
    expect(textoEstadoAcceso(fila("PERMITIDO"))).toBe("Puede entrar");
    expect(textoEstadoAcceso(fila("PRAIND_NO_REGISTRADO"))).toBe("Sin PRAIND registrada");
    expect(textoEstadoAcceso(fila("SIN_ACCESO", 20))).toBe("Acceso denegado");
    expect(textoEstadoAcceso(fila("EMPRESA_INACTIVA"))).toBe("Empresa inactiva");
    expect(textoEstadoAcceso(fila(null))).toBe("");
  });

  it("color según la gravedad", () => {
    expect(colorEstadoAcceso("PERMITIDO")).toBe("var(--exito)");
    expect(colorEstadoAcceso("PERMITIDO_CON_ADVERTENCIA")).toBe("var(--advertencia)");
    expect(colorEstadoAcceso("SIN_ACCESO")).toBe("var(--error)");
    expect(colorEstadoAcceso(null)).toBeUndefined();
  });
});
