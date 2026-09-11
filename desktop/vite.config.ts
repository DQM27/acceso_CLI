/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    // AG Grid Community (~870 KB, ver el grupo "ag-grid-vendor" abajo) ya
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
            // ~870 KB sin comprimir -- lo importa casi toda pantalla vía
            // Tabla.tsx) bajo el nombre de cualquier módulo chico que
            // cayera al lado alfabéticamente (visto como "Tabla-*.js",
            // escondiendo el tamaño real) -- confundía cualquier análisis
            // de bundle size futuro. Mismo fix ya aplicado en web/
            // (vite.config.ts), mismo criterio que "react-vendor".
            { name: "ag-grid-vendor", test: /node_modules[\\/]ag-grid-(community|react)[\\/]/ },
          ],
        },
      },
    },
  },
  clearScreen: false,
  server: {
    strictPort: true,
    // Sin esto, el watcher de Vite vigila `src-tauri/target` (donde cargo
    // escribe los .dll/.exe de cada rebuild) y en Windows choca con esos
    // archivos a mitad de escritura -- EBUSY, tira abajo `tauri dev` entero.
    // Mismo ignore que recomienda la plantilla oficial de Tauri+Vite.
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/vitest.setup.ts"],
  },
});
