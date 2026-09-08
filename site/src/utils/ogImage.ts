import { Resvg } from "@resvg/resvg-js";
import satori, { type Font } from "satori";

const LOGO_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect x="14" y="26" width="100" height="18" rx="9" fill="#d7dce3" opacity="0.24"/>
  <rect x="14" y="55" width="100" height="18" rx="9" fill="#d7dce3" opacity="0.24"/>
  <rect x="14" y="84" width="100" height="18" rx="9" fill="#d7dce3" opacity="0.24"/>
  <circle cx="27" cy="35" r="7.5" fill="#4ec98a"/>
  <circle cx="27" cy="64" r="7.5" fill="#f2687b"/>
  <circle cx="27" cy="93" r="7.5" fill="#6cb0f5"/>
  <rect x="44" y="30" width="58" height="10" rx="5" fill="#d7dce3"/>
  <rect x="44" y="59" width="40" height="10" rx="5" fill="#d7dce3"/>
  <rect x="44" y="88" width="50" height="10" rx="5" fill="#d7dce3"/>
</svg>`;

const LOGO = `data:image/svg+xml;base64,${Buffer.from(LOGO_SVG).toString("base64")}`;

async function googleFont(family: string, weight: number, text: string) {
  try {
    const css = await fetch(
      `https://fonts.googleapis.com/css2?family=${family}:wght@${weight}&text=${encodeURIComponent(text)}`,
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
  const [regular, bold] = await Promise.all([
    googleFont("Inter", 400, text),
    googleFont("Inter", 700, text),
  ]);
  const fonts: Font[] = [];
  if (regular) fonts.push({ name: "Inter", data: regular, weight: 400, style: "normal" });
  if (bold) fonts.push({ name: "Inter", data: bold, weight: 700, style: "normal" });
  cache = fonts;
  return fonts;
}

const FG = "#e6eaf0";
const DIM = "rgba(230,234,240,0.52)";

function chip(colour: string, label: string) {
  return {
    type: "div",
    props: {
      style: { display: "flex", alignItems: "center", gap: "9px", color: colour, fontSize: 21 },
      children: [
        {
          type: "div",
          props: {
            style: { width: "11px", height: "11px", borderRadius: "50%", background: colour },
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
      style: { display: "flex", flexDirection: "column", gap: "7px" },
      children: [
        {
          type: "div",
          props: {
            style: { display: "flex", gap: "16px", alignItems: "baseline", fontSize: 23 },
            children: [
              { type: "span", props: { style: { color: kindColour, fontWeight: 700 }, children: kind } },
              { type: "span", props: { style: { color: "rgba(230,234,240,0.62)" }, children: repo } },
              { type: "span", props: { style: { color: FG, fontWeight: 700 }, children: title } },
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
    ["[PR]", "#6cb0f5", "screenpipe/screenpipe#6449", "fix(meetings): lock live transcription",
      [["#4ec98a", "open"], ["#f2687b", "ci"]]],
    ["[ISSUE]", "#4dd0c8", "JuliaEarth/CoordRefSystems.jl#315", "Add support for ESPG:28992",
      [["#f2687b", "closed"]]],
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
          justifyContent: "space-between", padding: "64px 72px",
          background: "#0b0e14", fontFamily: fonts.length ? "Inter" : "sans-serif",
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
                    style: { display: "flex", alignItems: "center", gap: "18px" },
                    children: [
                      { type: "img", props: { src: LOGO, width: 56, height: 56, style: { display: "block" } } },
                      {
                        type: "span",
                        props: {
                          style: { fontSize: 44, fontWeight: 700, color: FG, letterSpacing: "-0.02em" },
                          children: "ghdeck",
                        },
                      },
                    ],
                  },
                },
                {
                  type: "div",
                  props: {
                    style: { fontSize: 60, fontWeight: 700, color: FG, lineHeight: 1.12, letterSpacing: "-0.02em", maxWidth: "940px" },
                    children: title,
                  },
                },
                {
                  type: "div",
                  props: { style: { fontSize: 26, color: DIM, lineHeight: 1.45, maxWidth: "880px" }, children: sub },
                },
              ],
            },
          },
          {
            type: "div",
            props: {
              style: {
                display: "flex", flexDirection: "column", gap: "20px",
                background: "#11151d", border: "1px solid #232935", borderRadius: "14px", padding: "26px 28px",
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
