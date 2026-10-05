/// <reference types="node" />
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { reglas } from "../reglas";

/** Misma huella que `reglas/wasm/build.rs`: FNV-1a de 64 bits sobre los
 * `Cargo.toml` y `src/*.rs` de `reglas/` y `reglas/wasm/`, en ese orden, con
 * la ruta relativa, un byte 0 y el contenido sin `\r`. */
function huellaDeLasFuentes(): string {
  // Vitest corre desde `web-visitas/` (bajo jsdom, `import.meta.url` no es una ruta).
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

  it("es la misma copia que usa el panel", () => {
    const propia = readFileSync(resolve("src/reglas/wasm/reglas_bg.wasm"));
    const panel = readFileSync(resolve("../web/src/reglas/wasm/reglas_bg.wasm"));
    expect(propia.equals(panel)).toBe(true);
  });

  it("normaliza el documento como el check-in de la portería", () => {
    expect(reglas.normalizarDocumento("01-0847-0293")).toBe("108470293");
    expect(reglas.normalizarDocumento(" ab-123 ")).toBe("AB123");
    expect(reglas.normalizarDocumento("AB")).toBeUndefined();
    expect(reglas.normalizarDocumento("A".repeat(21))).toBeUndefined();
  });
});
