import { describe, expect, it } from "vitest";
import { nombresCoinciden, problemaArchivo } from "./CarnePraind.logica";
import { urlWorker } from "../api/praind";

describe("nombresCoinciden", () => {
  it("ignora tildes, mayúsculas y un nombre de más", () => {
    expect(nombresCoinciden("Mindy  Murillo Guzmán", "MINDY MURILLO GUZMAN")).toBe(true);
    expect(nombresCoinciden("Ana María Pérez Soto", "ANA PEREZ SOTO")).toBe(true);
  });

  it("no coincide con otra persona ni con un solo nombre", () => {
    expect(nombresCoinciden("Ana Pérez Soto", "LUIS PEREZ SOTO")).toBe(false);
    expect(nombresCoinciden("Ana", "ANA PEREZ SOTO")).toBe(false);
    expect(nombresCoinciden(null, "ANA PEREZ")).toBe(false);
  });
});

describe("problemaArchivo", () => {
  it("acepta PNG/JPEG de hasta 10 MB", () => {
    expect(problemaArchivo({ size: 1000, type: "image/jpeg" })).toBeNull();
    expect(problemaArchivo({ size: 1000, type: "application/pdf" })).toMatch(/PNG o JPEG/);
    expect(problemaArchivo({ size: 11 * 1024 * 1024, type: "image/png" })).toMatch(/10 MB/);
  });
});

describe("urlWorker", () => {
  it("sigue al proyecto de Supabase del build, salvo que se indique otra", () => {
    expect(urlWorker(undefined, "https://xidaepyaljzkpbsxrqsm.supabase.co")).toBe("https://praind.megabrisas.com");
    expect(urlWorker(undefined, "https://pmrytjktlyiuikxuuxpr.supabase.co")).toBe(
      "https://praind-staging.megabrisas.com",
    );
    expect(urlWorker("http://localhost:8787/", "x")).toBe("http://localhost:8787");
  });
});
