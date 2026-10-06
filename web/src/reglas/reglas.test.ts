/// <reference types="node" />
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { reglas } from ".";

/** Misma huella que `reglas/wasm/build.rs`: FNV-1a de 64 bits sobre los
 * `Cargo.toml` y `src/*.rs` de `reglas/` y `reglas/wasm/`, en ese orden, con
 * la ruta relativa, un byte 0 y el contenido sin `\r`. */
function huellaDeLasFuentes(): string {
  // Vitest corre desde `web/` (bajo jsdom, `import.meta.url` no es una ruta).
  const raiz = resolve("../reglas");
  const archivos: string[] = [];
  for (const carpeta of ["", "wasm/"]) {
    archivos.push(`${carpeta}Cargo.toml`);
    const fuentes = readdirSync(resolve(raiz, `${carpeta}src`))
      .filter((nombre) => nombre.endsWith(".rs"))
      .sort();
    archivos.push(...fuentes.map((nombre) => `${carpeta}src/${nombre}`));
  }
  const PRIMO = 0x100000001b3n;
  const MASCARA = 0xffffffffffffffffn;
  let huella = 0xcbf29ce484222325n;
  const sumar = (byte: number) => {
    huella ^= BigInt(byte);
    huella = (huella * PRIMO) & MASCARA;
  };
  for (const archivo of archivos) {
    for (const byte of new TextEncoder().encode(archivo)) sumar(byte);
    sumar(0);
    for (const byte of readFileSync(resolve(raiz, archivo))) {
      if (byte !== 0x0d) sumar(byte);
    }
  }
  return huella.toString(16).padStart(16, "0");
}

describe("reglas del núcleo (WebAssembly)", () => {
  it("el paquete commiteado está al día con las fuentes de reglas/", () => {
    // Si falla: alguien cambió una regla en Rust y no regeneró el paquete.
    // Correr scripts/generar-reglas-wasm.sh y commitear lo generado.
    expect(reglas.huellaFuentes()).toBe(huellaDeLasFuentes());
  });

  it("responde con las reglas del núcleo", () => {
    expect(reglas.requierePraind("IN_HOUSE")).toBe(true);
    expect(reglas.requierePraind("SWAT")).toBe(false);
    expect(reglas.requierePraind("SWAT", true)).toBe(true);
    expect(reglas.requiereGafete("PRAIND")).toBe(true);
    expect(reglas.admitePersonalRuta("SWAT")).toBe(false);
    expect(reglas.tiposIngresoSeleccionables()).toEqual(["PRAIND", "IN_HOUSE", "SWAT"]);
    expect(reglas.normalizarCedula("01-1234-0567")).toBe("112340567");
    expect(reglas.normalizarCedula("AB123")).toBeUndefined();
  });

  it("valida y normaliza un contratista nuevo", () => {
    expect(
      reglas.validarContratista(
        { cedula: "1-1234-0567", nombre: " ana  pérez ", tipo_ingreso: "SWAT", fecha_vencimiento_praind: null },
        "2026-10-04",
      ),
    ).toEqual({
      ok: true,
      contratista: {
        cedula: "112340567",
        nombre: "ANA PÉREZ",
        tipo_ingreso: "SWAT",
        fecha_vencimiento_praind: null,
        es_personal_ruta: false,
        tiene_acceso: true,
      },
    });
    expect(
      reglas.validarContratista(
        { cedula: "112340567", nombre: "Ana", tipo_ingreso: "POR_CORREO", fecha_vencimiento_praind: null },
        "2026-10-04",
      ),
    ).toMatchObject({ ok: false, codigo: "tipo_ingreso_retirado" });
  });
});
