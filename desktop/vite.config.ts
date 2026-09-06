/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            // React se comparte entre pantallas y se puede cachear por separado.
            { name: "react-vendor", test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/ },
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
