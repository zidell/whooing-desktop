# Whooing Desktop

[후잉(Whooing)](https://whooing.com) 웹 가계부를 데스크톱 앱으로 감싼 프로젝트입니다. [Tauri](https://tauri.app)를 사용해 macOS/Windows/Linux용 네이티브 데스크톱 배포판을 만듭니다.

## 다운로드 및 설치

이 저장소의 [Releases](../../releases) 페이지에서 운영체제에 맞는 파일을 받아 설치하세요.

- **macOS**: `.dmg`
- **Windows**: [Microsoft Store](https://apps.microsoft.com/detail/9ND7MPSBWTL1)에서 설치 (아래 참고)
- **Linux**: `.deb`, `.rpm`, `.AppImage`

### Windows - Microsoft Store

Windows는 Microsoft Store를 통해 배포합니다. 아래 링크에서 설치하세요.

👉 **https://apps.microsoft.com/detail/9ND7MPSBWTL1**

Windows 기기에서는 아래 스토어 앱 딥링크로 바로 열 수도 있습니다.

```
ms-windows-store://pdp/?productid=9ND7MPSBWTL1
```

### macOS - Homebrew

[Homebrew](https://brew.sh)가 설치되어 있다면 다음 명령으로도 설치할 수 있습니다.

```sh
brew install --cask whooing
```

> Homebrew Cask 등록은 [homebrew-cask#275891](https://github.com/Homebrew/homebrew-cask/pull/275891) PR로 심사 중입니다. 머지 전까지는 위 Releases 페이지의 `.dmg`를 이용해 주세요.

### Linux

받은 패키지 파일을 배포판에 맞는 패키지 관리자로 설치하면 의존성이 자동으로 해결됩니다.

```sh
# Debian/Ubuntu 계열 (.deb)
sudo apt install ./Whooing_amd64.deb

# Fedora/RHEL 계열 (.rpm)
sudo dnf install ./Whooing-x86_64.rpm

# AppImage (배포판 무관, 설치 없이 바로 실행)
chmod +x Whooing_amd64.AppImage
./Whooing_amd64.AppImage
```

데스크톱 앱 없이도 웹 브라우저에서 바로 이용할 수 있습니다. 👉 **https://whooing.com**

## 개발·검증본 안내

q22에 연결하는 로컬 검증본과 whooing.com에 연결하는 배포앱의 실행 경로·빌드 설정·로그인 복귀 연결은 [개발·검증본과 배포앱 구분](docs/desktop-tabs.md#개발검증본과-배포앱-구분)에 정리했습니다. 로컬 검증본 재빌드는 Store 설치본 업데이트와 별개입니다. 두 실행본은 이름·아이콘·앱 식별자와 `whooing://` 스킴을 공유하므로 실행 경로와 서버를 확인하세요.

### Linux에서 직접 빌드한 실행본 등록

직접 빌드한 실행 파일을 앱 메뉴에 올릴 때는 바로가기를 손으로 만들지 말고 스크립트를 사용하세요.

```sh
npm run build                               # 또는 cargo build --release
./scripts/install-linux-dev-shortcut.sh     # 경로를 인자로 주면 다른 실행 파일도 등록
```

Wayland에서 창의 app_id는 실행 파일 이름(`whooing-desktop`)입니다. 바로가기의 `StartupWMClass`가 이 이름과 다르면 실행 직후 로고가 보였다가 창이 뜨면 "W" 글자 아이콘으로 바뀝니다. deb/rpm 패키지는 Tauri가 바로가기를 올바르게 만들어 주므로 해당하지 않습니다.

## 문의

후잉 서비스 관련 문의는 [whooing.com](https://whooing.com)을 통해 접수해 주세요.
