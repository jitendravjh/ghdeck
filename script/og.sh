#!/bin/sh
# Screenshots /og-card/ into site/public/og.png, the link preview image.
set -e
cd "$(dirname "$0")/../site"
npm run build >/dev/null
port=8931
(cd dist && exec python3 -m http.server "$port" >/dev/null 2>&1) &
server=$!
trap 'kill $server 2>/dev/null' EXIT
sleep 2
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu \
  --hide-scrollbars --force-color-profile=srgb --virtual-time-budget=6000 --window-size=1200,630 \
  --screenshot="$PWD/public/og.png" "http://localhost:$port/og-card/" 2>/dev/null
