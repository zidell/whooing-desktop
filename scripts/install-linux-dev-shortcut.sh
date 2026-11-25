#!/usr/bin/env bash
# 직접 빌드한 Linux 실행본을 앱 메뉴·작업표시줄에 등록하는 사용자 바로가기 설치.
# 사용: ./scripts/install-linux-dev-shortcut.sh [실행 파일 경로]
#       (기본: src-tauri/target/release/whooing-desktop)
#
# Wayland에서 창의 app_id는 실행 파일 이름(GTK prgname)이다. 바로가기의
# StartupWMClass가 이 이름과 다르면 KDE 등은 창과 바로가기를 연결하지 못해
# 실행 직후 로고가 보였다가 창이 뜨면 "W" 글자 아이콘으로 바뀐다.
# deb/rpm 번들은 Tauri가 StartupWMClass=<실행 파일 이름>으로 만들어 문제없으므로,
# 손으로 바로가기를 만들지 말고 이 스크립트를 쓴다.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
bin="$(realpath "${1:-$repo/src-tauri/target/release/whooing-desktop}")"
[ -x "$bin" ] || { echo "실행 파일이 없습니다: $bin (먼저 npm run build 또는 cargo build --release)" >&2; exit 1; }

wm_class="$(basename "$bin")"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
apps="$data/applications"
icon_dir="$data/icons/hicolor/256x256/apps"
desktop="$apps/com.whooing.desktop.desktop"

mkdir -p "$apps" "$icon_dir"
install -m 644 "$repo/src-tauri/icons/128x128@2x.png" "$icon_dir/$wm_class.png"

cat > "$desktop" <<EOF
[Desktop Entry]
Name=Whooing
Type=Application
Exec="$bin" %U
Icon=$wm_class
Terminal=false
Categories=Office;Finance;
StartupWMClass=$wm_class
MimeType=x-scheme-handler/whooing;
EOF

update-desktop-database "$apps" 2>/dev/null || true
gtk-update-icon-cache -q -t "$data/icons/hicolor" 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
xdg-mime default "$(basename "$desktop")" x-scheme-handler/whooing 2>/dev/null || true

echo "설치: $desktop"
echo "  Exec=$bin"
echo "  StartupWMClass=$wm_class"
echo "실행 중인 앱은 재시작해야 아이콘이 반영됩니다."
