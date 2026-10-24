import { describe, expect, it } from "vitest";
import { esPedidoDeAyuda, temasDeLaGuia } from "./guia";

describe("esPedidoDeAyuda", () => {
  it("reconoce las formas comunes de pedir ayuda, con o sin tildes y signos", () => {
    for (const texto of [
      "ayuda",
      "Ayuda!",
      "¿Qué podés hacer?",
      "que puedes hacer",
      "Qué sabe hacer usted?",
      "¿Qué le puedo pedir?",
      "que puedo preguntar aca",
      "cómo te uso",
      "¿para qué servís?",
      "opciones",
      "comandos",
    ]) {
      expect(esPedidoDeAyuda(texto), texto).toBe(true);
    }
  });

  it("no confunde un pedido de datos con un pedido de ayuda", () => {
    for (const texto of [
      "ayuda con los vencidos de ALDAMA",
      "¿qué PRAIND vencen el mes que viene?",
      "qué proveedores vinieron más veces",
      "opciones de filtro por empresa para los ingresos de esta semana",
      "mostrame los ingresos",
    ]) {
      expect(esPedidoDeAyuda(texto), texto).toBe(false);
    }
  });
});

describe("temasDeLaGuia", () => {
  it("sobre una vista abierta, primero las ideas para esa vista", () => {
    expect(temasDeLaGuia(true)[0].titulo).toBe("Sobre esta vista");
    expect(temasDeLaGuia(false).map((t) => t.titulo)).not.toContain("Sobre esta vista");
  });

  it("cada tema trae ejemplos, y los ejemplos no disparan la detección de ayuda", () => {
    for (const tema of temasDeLaGuia(true)) {
      expect(tema.ejemplos.length).toBeGreaterThan(0);
      for (const e of tema.ejemplos) expect(esPedidoDeAyuda(e), e).toBe(false);
    }
  });
});
