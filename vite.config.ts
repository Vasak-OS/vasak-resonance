import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "es2022",
    rollupOptions: {
      output: {
        // Con función y no con el objeto de nombres: Vite 8 empaqueta con
        // rolldown, que sólo acepta la forma de función. Los dos trozos son
        // los de antes: Vue con su enrutador y su almacén, y el desplazador.
        manualChunks(id: string) {
          if (/node_modules\/(?:vue|@vue\/[^/]+|vue-router|pinia)\//.test(id)) {
            return "vendor-vue";
          }
          if (id.includes("node_modules/vue-virtual-scroller/")) {
            return "vendor-virtual-scroller";
          }
          return undefined;
        },
      },
    },
  },
  define: {
    __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: false,
  },
}));
