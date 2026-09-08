import type { APIRoute } from "astro";
import { generateOgImage } from "../utils/ogImage";

export const GET: APIRoute = async () => {
  const png = await generateOgImage();
  return new Response(new Uint8Array(png), {
    headers: {
      "Content-Type": "image/png",
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });
};
