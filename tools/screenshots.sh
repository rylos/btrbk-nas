#!/bin/sh
# Takes the README screenshots into docs/img: the interface on made-up data
# (tools/demo.py), in headless Chromium, at twice the pixel density.
#
#   tools/screenshots.sh
set -eu

cd "$(dirname "$0")/.."
port=${PORT:-8767}  # PORT=... when 8767 is taken
out=docs/img
mkdir -p "$out"

python3 tools/devserver.py --demo --port "$port" >/dev/null 2>&1 &
server=$!
trap 'kill "$server"' EXIT
sleep 1

base="http://127.0.0.1:$port/index.html"
# shot NAME WIDTH,HEIGHT URL [chromium options]
shot() {
    name=$1 size=$2 url=$3
    shift 3
    LANG=en_US.UTF-8 LANGUAGE=en LC_ALL=en_US.UTF-8 chromium --headless=new --hide-scrollbars \
        --no-first-run --lang=en-US --force-device-scale-factor=2 --window-size="$size" \
        --virtual-time-budget=5000 --screenshot="$out/$name.png" "$@" "$url" >/dev/null 2>&1
    echo "$out/$name.png"
}

shot overview 1280,820 "$base#/t/0"
shot snapshots 1440,860 "$base?shot=queue#/t/0/g/@home"
newest=$(curl -s "http://127.0.0.1:$port/api.cgi/target?id=0" |
    python3 -c 'import json, sys; print(json.load(sys.stdin)["groups"][1]["summary"]["chain"]["next_parent"])')
shot browse 1280,760 "$base#/t/0/g/@home/b/$newest/alex/Documents"
shot settings 1280,900 "$base?shot=settings#/t/0"
shot dark 1280,820 "$base#/t/1" --force-dark-mode

# a 256-colour palette is plenty for flat interface colours, at a third of the size
if command -v pngquant >/dev/null; then
    pngquant --force --skip-if-larger --ext .png --quality 85-98 "$out"/*.png || true
fi
