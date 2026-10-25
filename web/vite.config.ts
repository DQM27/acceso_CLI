/// <reference types="vitest/config" />
import { defineConfig, searchForWorkspaceRoot } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // El esquema del lienzo vive en supabase/functions/_shared/lienzo (lo comparte
  // con la Edge Function `lienzo-ia`) y está escrito para Deno: importa
  // `npm:zod@…`. Acá ese import es el zod del panel.
  resolve: {
    alias: { "npm:zod@4.6.5": "zod" },
    dedupe: ["zod"],
  },
  server: {
    // La Ayuda lee `docs/manual/` con el lector compartido con el escritorio.
    fs: {
      allow: [
        searchForWorkspaceRoot(process.cwd()),
        "../supabase/functions/_shared/lienzo",
        "../supabase/functions/_shared/manual",
        "../docs/manual",
      ],
    },
  },
  build: {
    // AG Grid Community (~875 KB, ver el grupo "ag-grid-vendor" abajo) ya
    // está deliberadamente aislado en su propio chunk cacheable -- no es
    // bloat sin partir, es el tamaño real de la librería. Subir el límite
    // reconoce ese caso conocido en vez de que el build avise sobre él en
    // cada build; sigue avisando si aparece un chunk nuevo grande de
    // verdad por accidente (cualquiera por encima de este número).
    chunkSizeWarningLimit: 900,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            // React se comparte entre pantallas y se puede cachear por separado.
            { name: "react-vendor", test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/ },
            // Sin este grupo explícito, rolldown agrupaba AG Grid (pesado,
            // ~875 KB sin comprimir -- lo importan casi todas las pantallas
            // vía Tabla.tsx) bajo el nombre de cualquier módulo chico que
            // cayera al lado alfabéticamente (visto como
            // "useAutoRefresh-*.js" o "mensajeError-*.js" según qué otro
            // archivo cambiara) -- confundía cualquier análisis de bundle
            // size futuro. Nombre explícito, mismo criterio que
            // "react-vendor".
            { name: "ag-grid-vendor", test: /node_modules[\\/]ag-grid-(community|react)[\\/]/ },
          ],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/vitest.setup.ts"],
    // Sin esto, vitest también levanta e2e/panel.spec.ts (Playwright, no
    // vitest) -- mismo ajuste que ya tiene web-visitas/vite.config.ts.
    include: ["src/**/*.test.{ts,tsx}"],
    // Las pruebas de pantallas (montan formularios y la app) tardan 1-2 s solas,
    // pero con los ~40 archivos en paralelo la máquina va cargada y a veces
    // pasaban los 5 s por defecto: fallaban de forma intermitente sin ningún
    // error real (App.test, Usuarios.test). 15 s da margen sin esconder una
    // prueba colgada.
    testTimeout: 15_000,
  },
});
