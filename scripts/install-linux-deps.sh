#!/usr/bin/env bash
# Tauri v2 Linux 빌드에 필요한 시스템 패키지 설치.
# CI(.github/workflows/release.yml, ubuntu-22.04)와 동일한 패키지 목록.
# sudo로 실행: sudo ./scripts/install-linux-deps.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "root 권한이 필요합니다. sudo로 실행하세요: sudo $0" >&2
  exit 1
fi

apt-get update
apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
