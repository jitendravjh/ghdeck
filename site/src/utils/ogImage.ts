import { Resvg } from "@resvg/resvg-js";
import satori, { type Font } from "satori";

const LOGO_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect x="14" y="26" width="100" height="18" rx="9" fill="#e3e4e7" opacity="0.24"/>
  <rect x="14" y="55" width="100" height="18" rx="9" fill="#e3e4e7" opacity="0.24"/>
  <rect x="14" y="84" width="100" height="18" rx="9" fill="#e3e4e7" opacity="0.24"/>
  <circle cx="27" cy="35" r="7.5" fill="#4ec98a"/>
  <circle cx="27" cy="64" r="7.5" fill="#f2687b"/>
  <circle cx="27" cy="93" r="7.5" fill="#6cb0f5"/>
  <rect x="44" y="30" width="58" height="10" rx="5" fill="#e3e4e7"/>
  <rect x="44" y="59" width="40" height="10" rx="5" fill="#e3e4e7"/>
  <rect x="44" y="88" width="50" height="10" rx="5" fill="#e3e4e7"/>
</svg>`;

const LOGO = `data:image/svg+xml;base64,${Buffer.from(LOGO_SVG).toString("base64")}`;

async function googleFont(family: string, axes: string, text: string) {
  try {
    const css = await fetch(
      `https://fonts.googleapis.com/css2?family=${family}:${axes}&text=${encodeURIComponent(text)}`,
      { headers: { "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36" } },
    ).then((r) => r.text());
    const m = css.match(/src: url\((.+?)\) format\('(opentype|truetype)'\)/);
    if (!m) return null;
    const res = await fetch(m[1]);
    return res.ok ? await res.arrayBuffer() : null;
  } catch {
    return null;
  }
}

let cache: Font[] | null = null;

async function loadFonts(text: string): Promise<Font[]> {
  if (cache) return cache;
  const [serif, serifItalic, mono, monoBold] = await Promise.all([
    googleFont("IBM+Plex+Serif", "wght@400", text),
    googleFont("IBM+Plex+Serif", "ital,wght@1,400", text),
    googleFont("IBM+Plex+Mono", "wght@400", text),
    googleFont("IBM+Plex+Mono", "wght@600", text),
  ]);
  const fonts: Font[] = [];
  if (serif) fonts.push({ name: "Plex Serif", data: serif, weight: 400, style: "normal" });
  if (serifItalic) fonts.push({ name: "Plex Serif", data: serifItalic, weight: 400, style: "italic" });
  if (mono) fonts.push({ name: "Plex Mono", data: mono, weight: 400, style: "normal" });
  if (monoBold) fonts.push({ name: "Plex Mono", data: monoBold, weight: 600, style: "normal" });
  cache = fonts;
  return fonts;
}

const BG = "#111318";
const TEXT = "#abafb8";
const STRONG = "#e3e4e7";
const FAINT = "#878e9b";
const BLUE = "#90c5ff";
const LINE = "rgba(98,105,118,0.4)";

function chip(colour: string, label: string) {
  return {
    type: "div",
    props: {
      style: { display: "flex", alignItems: "center", gap: "9px", color: colour, fontSize: 20 },
      children: [
        {
          type: "div",
          props: {
            style: { width: "10px", height: "10px", borderRadius: "50%", background: colour },
          },
        },
        { type: "span", props: { children: label } },
      ],
    },
  };
}

function line(kind: string, kindColour: string, repo: string, title: string, chips: unknown[]) {
  return {
    type: "div",
    props: {
      style: { display: "flex", flexDirection: "column", gap: "8px" },
      children: [
        {
          type: "div",
          props: {
            style: { display: "flex", gap: "16px", alignItems: "baseline", fontSize: 21 },
            children: [
              { type: "span", props: { style: { color: kindColour }, children: kind } },
              { type: "span", props: { style: { color: FAINT }, children: repo } },
              { type: "span", props: { style: { color: STRONG, fontWeight: 600 }, children: title } },
            ],
          },
        },
        {
          type: "div",
          props: { style: { display: "flex", gap: "22px", paddingLeft: "6px" }, children: chips },
        },
      ],
    },
  };
}

export async function generateOgImage(): Promise<Uint8Array> {
  const title = "All your GitHub work in one list.";
  const sub = "PRs and issues together, newest first, with conflicts, failing CI and who you are waiting on.";
  const rows = [
    ["[PR]", BLUE, "screenpipe/screenpipe#6449", "fix(meetings): lock live transcription",
      [["#00c758", "open"], ["#ff6568", "ci"]]],
    ["[ISSUE]", "#5eead4", "JuliaEarth/CoordRefSystems.jl#315", "Add support for ESPG:28992",
      [["#ff6568", "closed"]]],
  ] as const;

  // google subsets to exactly the glyphs we ask for, so ask for everything we draw
  const subset = [
    title, sub, "ghdeck",
    ...rows.flatMap(([kind, , repo, rowTitle, chips]) => [
      kind, repo, rowTitle, ...chips.map(([, label]) => label),
    ]),
    "0123456789",
  ].join(" ");
  const fonts = await loadFonts(subset);

  const svg = await satori(
    {
      type: "div",
      props: {
        style: {
          width: "100%", height: "100%", display: "flex", flexDirection: "column",
          justifyContent: "space-between", padding: "60px 72px",
          backgroundColor: BG,
          backgroundImage: "linear-gradient(to top, rgba(28,57,142,0.32), rgba(28,57,142,0) 55%)",
          fontFamily: "Plex Mono",
        },
        children: [
          {
            type: "div",
            props: {
              style: { display: "flex", flexDirection: "column", gap: "22px" },
              children: [
                {
                  type: "div",
                  props: {
                    style: { display: "flex", alignItems: "center", gap: "16px" },
                    children: [
                      { type: "img", props: { src: LOGO, width: 50, height: 50, style: { display: "block" } } },
                      {
                        type: "span",
                        props: {
                          style: { fontFamily: "Plex Serif", fontSize: 40, color: STRONG, letterSpacing: "-0.01em" },
                          children: "ghdeck",
                        },
                      },
                    ],
                  },
                },
                {
                  type: "div",
                  props: {
                    style: {
                      fontFamily: "Plex Serif", fontStyle: "italic", fontSize: 66, color: BLUE,
                      lineHeight: 1.15, letterSpacing: "-0.02em", maxWidth: "1056px",
                    },
                    children: title,
                  },
                },
                {
                  type: "div",
                  props: { style: { fontSize: 22, color: TEXT, lineHeight: 1.5, maxWidth: "960px" }, children: sub },
                },
              ],
            },
          },
          {
            type: "div",
            props: {
              style: {
                display: "flex", flexDirection: "column", gap: "20px",
                background: "#07090d", border: `1px solid ${LINE}`, borderRadius: "2px", padding: "24px 28px",
              },
              children: [
                ...rows.map(([kind, colour, repo, rowTitle, chips]) =>
                  line(kind, colour, repo, rowTitle, chips.map(([c, label]) => chip(c, label))),
                ),
              ],
            },
          },
        ],
      },
    } as Parameters<typeof satori>[0],
    { width: 1200, height: 630, fonts },
  );

  return new Uint8Array(new Resvg(svg, { fitTo: { mode: "width", value: 1200 } }).render().asPng());
}
