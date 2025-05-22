/* app.config.ts */

import { defineConfig } from "@solidjs/start/config";
import devtools from "solid-devtools/vite";

export default defineConfig({
  vite: () => {
    return {
      plugins: [
        devtools({
          autostart: true,
        }),
      ],
    };
  },
});