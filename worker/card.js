// The link preview for /u/<username>. It draws the same thing the page opens with:
// their avatar, their name, and the counts, in the colours the site uses.

import { ImageResponse } from "workers-og";

const COUNTS = `
query($all:String!,$prs:String!,$iss:String!){
  all: search(query:$all,type:ISSUE,first:1){issueCount}
  prs: search(query:$prs,type:ISSUE,first:1){issueCount}
  iss: search(query:$iss,type:ISSUE,first:1){issueCount}
}`;

const esc = (s) =>
  String(s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

const tile = (label, value, dot) => `
  <div style="display:flex;flex-direction:column;flex:1;padding:20px 24px;margin-right:16px;border:1px solid #2b2f37;border-radius:8px">
    <div style="display:flex;align-items:center;font-size:23px;color:#878e9b">
      <div style="display:flex;width:12px;height:12px;border-radius:3px;background:${dot};margin-right:10px"></div>
      ${esc(label)}
    </div>
    <div style="display:flex;font-size:54px;font-weight:700;color:#f5f4f3;margin-top:4px">${esc(value)}</div>
  </div>`;

const card = ({ login, name, avatar, all, prs, iss }) => `
<div style="display:flex;flex-direction:column;width:1200px;height:630px;padding:58px 64px;background:#0e0f13;font-family:sans-serif">
  <div style="display:flex;align-items:center;font-size:30px;letter-spacing:1px;color:#878e9b">ghdeck</div>

  <div style="display:flex;align-items:center;margin-top:44px">
    ${avatar ? `<img src="${esc(avatar)}" width="150" height="150" style="border-radius:75px" />` : ""}
    <div style="display:flex;flex-direction:column;margin-left:34px">
      <div style="display:flex;font-size:60px;font-weight:700;color:#f5f4f3">${esc(name || login)}</div>
      <div style="display:flex;font-size:32px;color:#878e9b;margin-top:6px">@${esc(login)}</div>
    </div>
  </div>

  <div style="display:flex;margin-top:46px">
    ${tile("Involved in", all, "#5a92ee")}
    ${tile("Pull requests", prs, "#9a7deb")}
    ${tile("Issues", iss, "#00b34a")}
  </div>

  <div style="display:flex;margin-top:auto;font-size:26px;color:#626976">ghdeck.jitendravjh.in/u/${esc(login)}</div>
</div>`;

// the numbers come from issueCount, so nothing has to be paged through for a preview
export async function userCard(login, { profileOf, graphql, token }) {
  const [who, counts] = await Promise.all([
    profileOf(token, login).catch(() => null),
    graphql(token, COUNTS, {
      all: `involves:${login}`,
      prs: `involves:${login} is:pr`,
      iss: `involves:${login} is:issue`,
    }).catch(() => null),
  ]);
  if (!who) return null;

  const html = card({
    login: who.login,
    name: who.name,
    avatar: who.avatar ? `${who.avatar}${who.avatar.includes("?") ? "&" : "?"}s=300` : null,
    all: String(counts?.all?.issueCount ?? 0),
    prs: String(counts?.prs?.issueCount ?? 0),
    iss: String(counts?.iss?.issueCount ?? 0),
  });

  return new ImageResponse(html, { width: 1200, height: 630 });
}
