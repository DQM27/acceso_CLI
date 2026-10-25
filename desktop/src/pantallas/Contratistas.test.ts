import { describe, expect, it } from "vitest";
import { colorEstadoAcceso, columnasContratistas, textoEstadoAcceso } from "./Contratistas.logica";

const campos = columnasContratistas().map((c) => c.field);

describe("columnasContratistas", () => {
  it("nadie ve el interruptor 'Acceso': darlo o quitarlo es sólo del panel web", () => {
    expect(campos).not.toContain("tiene_acceso");
  });

  it("están las columnas base", () => {
    expect(campos).toEqual(
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
