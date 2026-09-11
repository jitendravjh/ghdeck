// @ts-check
import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://ghdeck.jitendravjh.in",
  integrations: [
    sitemap({ filter: (page) => !/\/(docs\/setup|docs\/use|og-card)\/?$/.test(page) }),
  ],
  redirects: {
    "/docs/setup/": "/docs/#connect-your-account",
    "/docs/use/": "/docs/#commands",
  },
  markdown: {
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark" },
      defaultColor: false,
      wrap: false,
    },
  },
});
