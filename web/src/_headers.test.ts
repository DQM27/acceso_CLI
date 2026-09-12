import { describe, expect, it } from "vitest";
// `?raw` (declarado por los tipos de `vite/client`, ya en tsconfig.json):
// trae el archivo tal cual, como string -- sin depender de `node:fs` (este
// proyecto no tiene tipos de Node instalados, es código de navegador puro
// salvo este test).
import contenido from "../public/_headers?raw";

/**
 * `public/_headers` (Cloudflare Workers/Pages lo aplica a todo lo que
 * sirve desde `dist/`, ver web/wrangler.jsonc) es la única defensa en
 * profundidad contra XSS que tiene este panel -- sin CSP, un XSS futuro
 * (ej. una dependencia de terceros comprometida) tiene el camino libre
 * para exfiltrar el JWT de sesión que Supabase guarda en `localStorage`.
 * Este test no reemplaza probarlo con un navegador real -- sólo evita que
 * alguien borre o afloje esto sin darse cuenta.
 */

describe("public/_headers", () => {
  it("define una Content-Security-Policy sin 'unsafe-inline' ni 'unsafe-eval' en script-src", () => {
    const linea = contenido.match(/^\s*Content-Security-Policy:\s*(.+)$/m)?.[1];
    if (!linea) throw new Error("No se encontró la línea Content-Security-Policy en _headers");
    const scriptSrc = linea.match(/script-src ([^;]+)/)?.[1];
    expect(scriptSrc).toBeDefined();
    expect(scriptSrc).not.toContain("unsafe-inline");
    expect(scriptSrc).not.toContain("unsafe-eval");
    expect(scriptSrc).toContain("'self'");
  });

  it("connect-src incluye el proyecto real de Supabase (REST/Auth y Realtime por wss)", () => {
    const linea = contenido.match(/^\s*Content-Security-Policy:\s*(.+)$/m)?.[1];
    if (!linea) throw new Error("No se encontró la línea Content-Security-Policy en _headers");
    const connectSrc = linea.match(/connect-src ([^;]+)/)?.[1] ?? "";
    expect(connectSrc).toContain("https://xidaepyaljzkpbsxrqsm.supabase.co");
    expect(connectSrc).toContain("wss://xidaepyaljzkpbsxrqsm.supabase.co");
  });

  it("bloquea que la app se embeba en un iframe ajeno (frame-ancestors + X-Frame-Options)", () => {
    expect(contenido).toContain("frame-ancestors 'none'");
    expect(contenido).toMatch(/X-Frame-Options:\s*DENY/);
  });

  it("tiene X-Content-Type-Options: nosniff", () => {
    expect(contenido).toMatch(/X-Content-Type-Options:\s*nosniff/);
  });

  it("default-src y base-uri quedan cerrados -- cada tipo de recurso se permite a propósito, no por default", () => {
    const linea = contenido.match(/^\s*Content-Security-Policy:\s*(.+)$/m)?.[1];
    if (!linea) throw new Error("No se encontró la línea Content-Security-Policy en _headers");
    expect(linea).toContain("default-src 'none'");
    expect(linea).toContain("base-uri 'none'");
  });

  it("style-src permite unsafe-inline (AG Grid inyecta <style> dinámicos -- ver docs/decisiones-tecnicas.md) pero nunca unsafe-eval", () => {
    const linea = contenido.match(/^\s*Content-Security-Policy:\s*(.+)$/m)?.[1];
    if (!linea) throw new Error("No se encontró la línea Content-Security-Policy en _headers");
    const styleSrc = linea.match(/style-src ([^;]+)/)?.[1];
    expect(styleSrc).toBeDefined();
    expect(styleSrc).not.toContain("unsafe-eval");
  });

  it("no deja que un buscador indexe el panel ni que el navegador cachee sus datos", () => {
    expect(contenido).toMatch(/X-Robots-Tag:\s*noindex/);
    expect(contenido).toMatch(/Cache-Control:\s*no-store/);
  });

  it("Referrer-Policy y Cross-Origin-Opener-Policy están al máximo (no hay otro origen que necesite el referrer ni una ventana propia)", () => {
    expect(contenido).toMatch(/Referrer-Policy:\s*no-referrer/);
    expect(contenido).toMatch(/Cross-Origin-Opener-Policy:\s*same-origin/);
  });
});
