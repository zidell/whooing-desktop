# 프로젝트 개요

후잉(Whooing) 웹앱을 **Tauri**로 래핑해서 데스크톱 배포 채널에 등록하는 프로젝트.

- 대상 채널: 다이렉트 다운로드(macOS/Windows/Linux), Microsoft Store, Linux(Snap Store/Flathub), Homebrew Cask
- **모바일은 스코프 밖**:
  - 안드로이드는 이미 TWA(Bubblewrap)로 별도 완료·배포됨(패키지명 `com.whooing.app`, Play 앱 서명 적용 중). Tauri로 다시 만들 이유 없음.
  - iOS 앱스토어 + 맥앱스토어(Apple Silicon)는 애플 심사 리스크(4.2류 "최소 기능" 리젝) 때문에 별도 Xcode 네이티브 트랙으로 진행 중. Tauri 2.x가 모바일을 지원하게 됐어도 이 분리는 유지 — 리스크의 원인이 툴체인이 아니라 "순수 웹뷰 래퍼"라는 콘텐츠 자체의 문제라서, Tauri로 iOS를 빌드해도 동일한 리스크가 남음.
- 후잉은 이미 PWA가 완성돼 실사용 중. 목표는 순수 "여러 마켓에 등록"이지 기능 재구현이 아님.

## 메인 웹 서비스 레포와의 관계
- 메인 웹 서비스(whooing.com) 레포에 원본 전략 문서(Tauri 래핑 전략, 배포 채널별 심사 특성 비교, 다운로드 페이지 구조 등)가 있고, 이 레포는 그 문서의 실행 본체.
- 웹 레포는 서빙 자산(manifest.json, assetlinks.json 등)만 소관하는 기존 패턴(Android/iOS와 동일) — **실제 Tauri 프로젝트 소스, CI, 빌드/릴리즈는 전부 이 레포 안에서 진행**. "배포 전용" 저장소가 아니라 개발+빌드+배포가 다 여기서 일어남.
- 웹 레포와의 유일한 연동 지점: 웹 서비스의 데스크톱 다운로드 리다이렉트 로직이 현재 `dl.todesktop.com`(옛 Electron 빌드)으로 향하고 있는데, 이걸 이 레포의 GitHub Release asset URL로 교체할 예정(그 작업 자체는 웹 레포 쪽에서 진행, 여기서 직접 건드리지 않음).

# 빌드 정책 (중요)

- **Dev 빌드 불필요. 항상 production 빌드만 생성한다** — 사용자 명시 지침.
- `tauri.conf.json` 등에서 별도 dev/staging 채널 구분 없이 release 빌드 하나로 통일. 로컬 `tauri dev`는 코드 확인용으로만 쓰고, 배포용 산출물은 항상 release 모드로 빌드.
- CI: `tauri-apps/tauri-action`(공식 GitHub Action)으로 `macos-latest` / `windows-latest` / `ubuntu-22.04` 3-way 매트릭스. 버전 태그(`v*.*.*`) push 시 자동 빌드 → GitHub Release 초안에 3개 OS 아티팩트 자동 첨부.
- OS별 산출물:
  - **macOS**: `.dmg`(+`.app`) — Developer ID 서명 + `notarytool` 노터라이즈 필수.
  - **Windows**: `.msi`(WiX) 또는 `.exe`(NSIS) — 코드서명 인증서는 선택.
  - **Linux**: `.deb`, `.rpm`, `.AppImage` — Tauri 번들러 기본 자동 생성.

# 다이렉트 다운로드 호스팅

- **GitHub Releases**로 호스팅(이 레포). Homebrew Cask가 어차피 공개 GitHub 릴리즈 URL을 요구하므로 다이렉트 다운로드와 인프라 공유.
- 웹 레포 연동은 두 방식 중 택1(실행 단계에서 재검토):
  - A: 웹 서버가 GitHub API `releases/latest`를 캐싱해서 조회 후 리다이렉트
  - B: 배포 스크립트가 릴리즈 시점에 고정 URL을 웹 서버 설정에 갱신

# 마켓 등록 전략 (맥앱스토어 제외)

- **Microsoft Store**: Tauri 기본 산출물(msi/nsis)은 스토어 제출 포맷이 아님 → MSIX 필요. PWABuilder로 별도 생성하는 게 가장 간단(대안: MSIX Packaging Tool로 수동 래핑, 비권장). 스토어가 자체 서명하므로 코드서명 인증서 구매 불필요. 심사는 관대(기능성 리젝 없음).
- **Linux**: Snap Store 우선 진행(`snapcraft.yaml`, 무료 계정, 진입장벽 낮음) → Flathub는 후순위(소스빌드 매니페스트 요구, 리뷰 오래 걸림).
- **Homebrew Cask**: `homebrew-cask` 저장소에 formula PR. 요건은 서명+노터라이즈된 macOS 앱의 안정적 다운로드 URL(=이 레포 GitHub Release 자산) + sha256. `livecheck` 블록으로 버전 자동 추적 가능.

# 실행 순서

1. Tauri 프로젝트 세팅 + CI 매트릭스 빌드 (선행 조건 — 나머지 전부 이 산출물에 의존)
2. macOS 서명/노터라이즈 확보 → macOS 다이렉트 다운로드 먼저 검증
3. 웹 레포의 데스크톱 다운로드 리다이렉트 대상을 GitHub Release로 교체 → todesktop.com 의존 제거
4. Homebrew Cask 등록 (macOS 빌드만 있으면 바로 가능, 가장 간단)
5. Windows: PWABuilder vs MSIX 직접 패키징 결정 → MS Store 제출
6. Linux: Snap Store 먼저 → Flathub는 후순위

# 진행 현황

- [x] GitHub 저장소 생성 (`zidell/whooing-desktop`, public)
- [x] 로컬 개발 환경 세팅
- [x] Tauri 프로젝트 초기 세팅 — `app.windows[0].url`을 `https://whooing.com`으로 지정하는 원격 URL 래퍼 구조. 앱 아이콘은 실제 후잉 로고(`logo_app_1024.png`) 적용 완료
- [x] CI 매트릭스(macOS/Windows/Linux) 빌드 파이프라인 구성 (`.github/workflows/release.yml`, tauri-action, 태그 `v*.*.*` push 트리거) — `workflow_dispatch`로 3-way 매트릭스 전부 그린 확인(.dmg/.app.tar.gz, .exe/.msi, .deb/.rpm/.AppImage 산출물 정상 생성). 검증용 draft release는 정리함
- [ ] macOS 코드서명/노터라이즈 시크릿 등록 (`APPLE_CERTIFICATE` 등 GitHub Secrets) — 현재 macOS 산출물은 미서명 상태로만 빌드됨
- [ ] Windows 코드서명 여부 결정 (선택 사항)
- [ ] 웹 레포 데스크톱 다운로드 리다이렉트 → GitHub Release 연동으로 교체
- [ ] Homebrew Cask formula 등록
- [ ] Windows: PWABuilder vs MSIX 결정 → MS Store 제출
- [ ] Linux: Snap Store 제출
- [ ] Linux: Flathub 제출 (후순위)

# 참고

- 전체 전략 논의 원본(채널별 심사 특성 비교표, 웹 레포 다운로드 페이지 구조 상세 등)은 메인 웹 서비스 레포 쪽 문서에 있음(내부 문서, 이 공개 레포에는 포함하지 않음).
