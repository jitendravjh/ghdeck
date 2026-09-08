import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) => {
  const base = site ?? new URL("https://ghdeck.jitendravjh.in");
  const body = [
    "User-agent: *",
    "Allow: /",
    "",
    `Host: ${base.host}`,
    `Sitemap: ${new URL("/sitemap-index.xml", base).href}`,
  ].join("\n");

  return new Response(body, {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
};
