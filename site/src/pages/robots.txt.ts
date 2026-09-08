import type { APIRoute } from "astro";

// Cloudflare prepends its own managed block, including a `User-agent: *` group,
// so this only adds the sitemap pointer it leaves out.
export const GET: APIRoute = ({ site }) => {
  const base = site ?? new URL("https://ghdeck.jitendravjh.in");
  const body = `Sitemap: ${new URL("/sitemap-index.xml", base).href}\n`;

  return new Response(body, {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
};
