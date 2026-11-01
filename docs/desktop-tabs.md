# 데스크톱 멀티탭

Windows·macOS·Linux 모두 로컬 탭바 웹뷰와 콘텐츠용 child webview를 사용하는 공통 구현이다. 탭이 하나면 탭바와 해당 공간을 숨기고 두 개 이상이면 높이 32 논리 픽셀로 표시한다. Windows/macOS는 콘텐츠 좌표·높이를 조정하고 Linux는 GTK box packing과 높이를 제한한 shell 컨테이너의 표시 여부를 조정한다. 탭 색상은 OS 라이트/다크 설정을 따른다.

주소표시줄과 뒤로·앞으로 버튼은 없다. 새 탭, 닫기, 복제, 닫은 탭 다시 열기, 드래그 순서 변경, 가로 스크롤과 활성 탭 자동 노출을 제공한다. 우클릭 메뉴에는 복제·좌우 이동·닫기·다른 탭 닫기·오른쪽 탭 닫기·닫은 탭 다시 열기가 있다. 가운데 클릭으로 닫을 수 있다. 마지막 탭을 닫으면 창도 닫힌다. 복제는 현재 콘텐츠 URL을 읽어 원본 바로 뒤에 추가한다. 닫은 탭은 메모리에서 최대 20개까지 원래 위치로 복원하며 인증·결제 URL은 복구 대상에서 제외한다.

아래 Primary는 macOS의 Cmd, Windows/Linux의 Ctrl이다.

| 동작 | 단축키 |
| --- | --- |
| 새 탭 / 닫기 | Primary+T / Primary+W |
| 복제 / 닫은 탭 다시 열기 | Primary+Shift+D / Primary+Shift+T |
| 1~8번째 / 마지막 탭 | Primary+1~8 / Primary+9 |
| 다음 / 이전 탭 | Ctrl+Tab / Ctrl+Shift+Tab (모든 OS) |
| 다음 / 이전 탭 (Mac 추가) | Cmd+Shift+] / Cmd+Shift+[ |
| 이전 / 다음 탭 | Primary+PageUp / Primary+PageDown |
| 탭 좌우 이동 | Primary+Shift+PageUp / Primary+Shift+PageDown |
| 새로고침 | Primary+R |
| 화면 확대 / 축소 / 100% 복원 | Primary++ (또는 Primary+=) / Primary+- / Primary+0 |

macOS Cmd+Tab은 시스템 앱 전환으로 유지한다. macOS는 기본 앱 메뉴의 탭 단축키와 WKWebView 방문 기록 스와이프도 연결한다. Windows/Linux 콘텐츠의 Alt+좌우 방문 기록 이동을 유지한다. 탭바에서는 좌우/Home/End로 선택하고 Shift+F10으로 메뉴를 열 수 있다. 탭 하나일 때도 단축키와 새 창 링크로 탭을 추가할 수 있다.

일반 새 창 요청은 앱 탭으로 연다. 후잉 내부 일반 이동은 현재 탭, 후잉에서 외부 사이트로 이동하면 새 탭, 외부 탭 내 일반 이동은 현재 탭으로 처리한다. OAuth·SNS 로그인·결제 export_to·mailto는 기본 브라우저/메일 앱을 사용한다. 웹사이트의 명시적 `open_external` 요청도 브라우저 전달을 유지한다. 수동 생성 탭은 opener 관계를 보존하지 않으며 window.open 반환값은 null이다.

URL 정책은 `link_action`에서 공유한다. 탭 관리 IPC는 로컬 main 웹뷰만 사용할 수 있다. 후잉 콘텐츠에는 기존 알림/브라우저 전달 권한만 주며 다른 사이트에는 IPC 권한을 주지 않는다. 콘텐츠의 탭 단축키는 활성 탭에서만 제한된 전용 스킴으로 처리한다. WebView2 콜백에서 동기 생성할 때의 교착을 피하기 위해 탭 생성·전환·닫기는 별도 작업 스레드에서 직렬 처리한다.

## 개발·검증본과 배포앱 구분

2026-10-25 이 Windows 컴퓨터에서는 아래 두 실행본을 구분해서 사용한다. 이번 멀티탭 변경을 빌드·실행한 것은 로컬 검증본이며, 기존 Store 패키지를 업데이트하거나 교체한 것이 아니다.

| 구분 | 로컬 개발·검증본 | 실제 배포앱 |
| --- | --- | --- |
| 연결 서버 | `https://q22.whooing.com` | `https://whooing.com` |
| 실행 경로 | 이 저장소의 `src-tauri/target/debug/whooing-desktop.exe` | Microsoft Store에서 설치한 앱 |
| 설정 | `src-tauri/tauri.preview.conf.json`을 적용해 빌드 | 기본 배포 설정 |
| 변경 반영 | 소스 수정 후 검증본 재빌드·재실행 | 새 버전 배포 후 설치·업데이트 |
| Windows 기본 앱 후보 이름 | `후잉 (로컬 테스트)` | Store 설치 앱 |

**실행 파일과 연결 서버를 나눈 상태이며 앱 식별자를 완전히 분리한 구성은 아니다.** preview 설정은 productName·identifier·아이콘·실행 파일명을 바꾸지 않는다. 창 제목과 프로세스명만으로는 두 실행본을 구분하기 어렵다. 단일 인스턴스 처리와 `whooing://` 스킴도 공유하므로, 독립적인 두 앱의 동시 실행·로그인 상태 분리까지 보장하지 않는다. 검증할 때는 사용할 실행 경로와 서버를 확인한다. q22 화면의 `Q22 VERSION` 표시는 검증 서버를 확인하는 보조 수단이다.

로그인 복귀에 쓰는 `whooing://`의 기본 앱은 Windows 설정에서 선택한다. 이 컴퓨터에서는 검증을 위해 `후잉 (로컬 테스트)`를 선택했다. 따라서 브라우저 로그인 후 검증본으로 돌아오는 상태다. 배포앱 사용으로 전환할 때는 기본 앱 연결도 Store 앱으로 되돌려야 한다. 일반 앱 실행용 기존 Store 바로가기와 검증본 실행 경로를 혼동하지 않는다.

## 이 컴퓨터의 q22 검증본 빌드

PowerShell에서:

```powershell
$env:TAURI_CONFIG = Get-Content src-tauri/tauri.preview.conf.json -Raw
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

같은 셸에서 기본 배포 설정으로 다시 빌드할 때는 먼저 `Remove-Item Env:TAURI_CONFIG -ErrorAction SilentlyContinue`로 preview 설정을 해제한다. 같은 debug 출력 경로를 다시 빌드하면 기존 실행 파일이 바뀔 수 있으므로, 경로만 보고 연결 서버를 단정하지 않는다.

실행 파일은 `src-tauri/target/debug/whooing-desktop.exe`다. preview 설정은 시작 주소를 `https://q22.whooing.com`으로 지정한다. 새 후잉 탭과 OAuth 딥링크 복귀도 같은 서버를 사용한다. 설정하지 않은 배포 빌드의 기본 주소는 `https://whooing.com`이다. 시작 주소는 자격증명·비표준 포트 없는 HTTPS 후잉 도메인으로 제한한다.

Windows 개발 빌드는 기존 동작대로 whooing:// 핸들러를 자신에게 등록한다. 명시적인 localPreview 설정이 있을 때만 별도 ProgID와 RegisteredApplications 후보 ‘후잉 (로컬 테스트)’도 등록한다. 기본 링크 앱 선택은 Windows 설정 화면에서 수행하며 UserChoice를 직접 쓰지 않는다. 로컬 검증본 실행과 Store 패키지 설치는 별개이며 Store 패키지는 교체하지 않았다.

로컬 실행은 탐색기나 Win+R에서 일반 권한으로 한다. 관리자 권한 개발 도구가 직접 시작하면 후잉도 High integrity를 상속한다. Chrome의 Medium integrity에서 실행 중인 High integrity 앱으로 보내는 single-instance WM_COPYDATA가 UIPI에 차단되어 로그인 복귀 URL이 유실된다. Windows 메시지 필터를 완화하지 않고 후잉을 일반 권한으로 실행해 해결했다.

## 검증

2026-10-25 Windows/WebView2에서 일반 target=_blank, window.open, 우클릭 ‘새 창에서 링크 열기’, Ctrl+Shift+Tab, Ctrl+W, 최대화 시 크기 조정을 실제 화면에서 확인했다. `tests/manual/tab-links.html`로 외부 콘텐츠의 tabs_snapshot, tabs_action, badge, attention, open_external 호출이 모두 거부되는 것도 확인했다. 이 검증 페이지는 개발 중 임시 주소 입력 UI로 열었으며 최종 앱에는 그 UI가 없다.

q22 접근 거부는 사용자가 기기의 Tailscale IP를 허용한 뒤 해소됐다. 일반 권한으로 실행한 Windows 앱에서 q22 Google 로그인 → Chrome 로그인 완료 → 기존 후잉 탭의 대시보드 복귀까지 실제 화면에서 확인했다. 임시 진단에서 관리자 실행 때는 두 번째 프로세스 실행만 기록됐고, 일반 실행에서는 딥링크 이벤트·기존 탭 선택·navigate 성공까지 기록됐다. 진단은 인증값이나 URL을 기록하지 않았으며 원인 확인 후 코드를 제거했다.

최종 Windows 실행본에서 숫자 직접 선택(1·2·9), Ctrl+Tab/Shift+Tab, 현재 게시판 페이지 복제, 드래그 정렬, 닫기와 복구, 우클릭 오른쪽 탭 일괄 닫기와 원래 순서 복구를 확인했다. 한 탭에서 탭바와 빈 공간 제거, 두 탭 이상에서 32px 탭바를 확인했다. Windows 설정에서 라이트로 전환한 후 실행 중 탭 색상이 즉시 변경되는 것을 화면으로 확인하고 기존 다크 모드로 복원했다.

Rust 테스트 11개(링크·도메인·OAuth·단일 탭바·숫자 선택·순서 변경·복제/복구 URL 제한·탭 DTO)와 Node 단축키 테스트 4개(Windows/Linux/macOS 키 조합)가 통과했다. JavaScript 문법 검사와 Windows 빌드도 통과했다. macOS/Linux는 플랫폼 구현과 라이브러리 API를 읽어 확인했으며 이 Windows 컴퓨터에서 실행 검증하지 않았다.

2026-10-25 macOS에서 release 빌드를 `/Applications/Whooing.app`에 설치해 확인했다. Cmd+T로 탭은 생성되지만 콘텐츠가 탭바를 가리는 현상이 있었다. 실제 WKWebView 좌표를 계측한 결과 콘텐츠에는 요청한 32px 탭바 높이가 적용됐지만, 로컬 탭바 HTML이 피하는 제목 표시줄의 safe area 28px를 콘텐츠 배치에서 빠뜨렸다. macOS는 네이티브 부모 뷰의 크기와 `safeAreaInsets().top`을 기준으로 콘텐츠 위치·높이를 계산하도록 수정했고, 두 탭에서 탭바와 웹페이지가 겹치지 않는 것을 실제 화면으로 확인했다. Windows/Linux의 기존 위치 계산은 유지한다. 로컬 release 빌드도 macOS 자동 업데이트 대상이며, 원격 릴리즈 설치 후 필요할 때 로컬 소스를 다시 빌드·설치하는 방식으로 사용한다.

같은 macOS 실행본에서 탭 선택 시 제목·닫기 버튼이 약 16px 이동하는 현상도 확인했다. 탭의 flex basis만 지정하면 탭 목록의 내용 기준 너비가 실제 탭 너비보다 작게 계산돼, 두 탭만 있어도 선택에 따라 가로 스크롤이 생겼다. 공통 CSS에 탭 너비 180px를 명시하고 제목은 별도 span에서 왼쪽 정렬·말줄임 처리한다. 선택 시 자동 스크롤은 DOM 갱신이 끝난 뒤 탭 목록의 scrollLeft만 조정하며, 제목이 같으면 텍스트 노드를 다시 쓰지 않는다. 수정본을 리빌드·설치하고 Cmd+1/Cmd+2로 번갈아 선택해 탭 버튼 중심 946/1126과 닫기 버튼 중심 1034/1214가 유지되는 것을 desk 화면으로 확인했다. Windows/Linux는 같은 CSS·JavaScript 경로를 사용하며 실행 검증은 macOS에서 수행했다.


## Linux 탭바가 콘텐츠를 아래로 미는 문제

2026-11-01 KDE Wayland/WebKitGTK에서 한 탭은 정상이고, 두 번째 탭을 열면 탭바 아래에 큰 빈 공간이 생기거나 콘텐츠가 창 아래로 밀리는 문제를 재현했다. GTK 할당 시점의 측정에서 shell의 preferred height는 최소 32px·natural 890px였고, 실제로 752px를 배정받았다. GtkBox packing의 expand/fill은 모두 false였으며 실제 부모도 해당 GtkBox였다. `set_size_request(-1, 32)`는 최소 높이만 지정하므로 WebKitGTK가 제안하는 큰 natural height를 제한하지 못했다.

Linux shell을 최소·최대 콘텐츠 높이 32px인 GtkScrolledWindow에 넣고 natural height 전파에 상한을 적용한다. 스크롤바는 표시하지 않고, 탭 하나일 때는 컨테이너까지 숨긴다. 콘텐츠 웹뷰만 나머지 공간을 확장해 받는다. 컨테이너는 세로 박스의 첫 위치에 한 번만 추가하며 창 크기 변경이나 탭 추가·닫기에서 재사용한다. 임시 측정 코드는 배포본에 포함하지 않는다.

이 원인은 Linux에서 여러 웹뷰를 GtkBox에 쌓으며 웹뷰의 natural height를 제한하지 않는 다른 Tauri 앱에도 적용될 수 있다. 단일 웹뷰 앱이나 모든 Tauri 앱에 발생한다고 일반화할 근거는 없다. 의존 라이브러리 Wry 0.55.1과 0.57.0 모두 GtkBox 부모에는 `pack_start(webview, true, true, 0)`를 사용하고, 좌표 기반 `set_bounds`는 GtkFixed 부모에서만 적용한다. 따라서 라이브러리 버전만 바꾸는 방식으로 해결하지 않는다.

Windows/WebView2는 `SetBounds`와 `SetWindowPos`, macOS/WKWebView는 부모 좌표계에 맞춘 `setFrame`으로 콘텐츠 경계를 적용한다. 후잉은 두 OS에서 탭바 32px를 뺀 콘텐츠 영역을 직접 계산하며 macOS에서는 제목 표시줄 safe area도 포함한다. 이번 GTK natural height 원인이 두 OS에 적용되지는 않는다. 이번 작업의 실행 검증 대상은 Linux이며 Windows/macOS는 앱 코드와 Wry 구현, 앞 절의 기존 실행 검증 기록을 확인한 범위다.


## Linux Ctrl+Tab / Ctrl+Shift+Tab

2026-11-01 Linux 실행본에서 마우스로 탭을 선택하면 전환되지만 콘텐츠에 포커스가 있을 때 Ctrl+Tab과 Ctrl+Shift+Tab은 모두 전환되지 않는 것을 재현했다. 기존 JavaScript 키 판정 테스트는 통과했으므로 DOM 이벤트의 판정 함수만으로 실제 입력 경로를 보장할 수 없었다. Linux는 GTK host window의 key-press-event에서 두 조합을 받아 기존 탭 작업 큐의 Cycle로 전달하고 이벤트를 소비한다. 따라서 웹뷰·탭바·입력칸 포커스와 관계없이 같은 전환 경로를 사용하며 JavaScript와 중복 처리하지 않는다.

역방향은 Shift modifier뿐 아니라 XKB의 ISO_Left_Tab keysym도 처리한다. 일반 Tab/Shift+Tab과 Alt·Super·Meta가 추가된 조합은 가로채지 않고 Caps Lock 등 상태 비트는 허용한다. Windows/macOS의 기존 단축키 경로는 유지한다.


최종 Linux release 실행본에서 탭 1개일 때 탭바·여백 제거, 탭 2개 이상에서 32px 탭바와 정상 콘텐츠 영역, 마우스 탭 선택, 최대화, 탭 닫기와 재생성을 화면으로 확인했다. 단축키 수정 후 세 탭에서 Ctrl+Shift+Tab의 3→2 전환과 Ctrl+Tab의 2→3 전환, 검색 입력칸에 포커스를 둔 상태의 전환도 확인했다. Rust 테스트 12개와 Windows/Linux/macOS 키 판정 Node 테스트 4개가 통과했다. 이 기기에 cargo-fmt가 없어 자동 포맷 검사는 실행하지 못했으며 빌드와 diff 공백 검사는 통과했다.

이 Linux 기기의 개발 소스는 `~/Sites/whooing-desktop`에서 수정하고, 기존 바로가기 `com.whooing.desktop.desktop`가 가리키는 `~/Projects/whooing-desktop/src-tauri/target/release/whooing-desktop`에 최종 release 바이너리를 반영했다. 실제 실행 프로세스와 `whooing://` 핸들러가 해당 Projects 실행본을 사용하는 것도 확인했다. 임시 검증용 바로가기와 높이 측정 로그는 제거했다.


## 화면 확대·축소

확대 단축키는 콘텐츠와 로컬 탭바의 공통 키 판정에서 받아 기존 탭 작업 큐를 거쳐 모든 콘텐츠 웹뷰의 `set_zoom`을 호출한다. 일반 키보드의 +·=·-와 숫자 키패드의 +·-를 지원한다. 탭바 크기는 유지하며 콘텐츠를 10%p씩 50~300% 범위에서 조절하고 Primary+0으로 100%로 복원한다. 배율은 앱 전체에 하나로 적용하며 변경 즉시 OS별 앱 설정 디렉터리의 `zoom.json`에 저장한다. 새 탭·복제·닫은 탭 복구와 앱 재실행에도 저장한 배율을 적용한다. Ctrl+0으로 복원한 100%도 저장한다. 설정 파일이 없거나 값이 잘못됐으면 100%로 시작한다. Linux·macOS·Windows 모두 같은 저장·읽기 경로를 사용하며 앱 설정 디렉터리 위치만 Tauri가 OS별로 선택한다.

Tauri 기반 Wry의 확대 단축키 옵션은 기본 false이고 Windows에만 적용된다. Linux/WebKitGTK와 macOS/WKWebView에는 해당 옵션이 구현돼 있지 않으므로, 세 OS 공통으로 단축키를 명시적으로 연결한다. 네이티브 확대 API는 각각 WebKitGTK `set_zoom_level`, WebView2 `SetZoomFactor`, WKWebView `setPageZoom`을 사용한다.

2026-11-01 확대·축소 변경의 Rust 테스트 12개, Windows/Linux/macOS 공통 키 판정 Node 테스트 4개, JavaScript 문법 검사와 Linux release 빌드가 통과했다. 화면 인식 중 로컬 Tesseract 시간 초과가 발생했고 이후 사용자 컴퓨터 제어 중지 신호를 받아 UI 검증을 중단했다. 이 변경은 Sites 소스에 반영했으며 Projects의 기존 실행본 교체·재시작은 수행하지 않았다. Windows/macOS는 공통 단축키 경로와 네이티브 확대 API 구현을 코드로 확인한 범위다.


같은 날 사용자 요청으로 설치본 검증을 재개했다. Sites 소스의 최종 release 빌드를 다시 확인하고 기존 Projects 실행 파일에 설치했다. 기존 바로가기 `com.whooing.desktop.desktop`로 재실행했으며 `/proc/<pid>/exe` 경로와 SHA-256이 빌드 결과와 일치하는 것을 확인했다. `whooing://` 기본 핸들러도 같은 바로가기다.

Linux/KDE Wayland의 실제 후잉 화면에서 Ctrl+=로 110%, Ctrl+Shift+=로 120%, Ctrl+-로 110%, Ctrl+0으로 100%, 다시 Ctrl+-로 90%가 되는 화면 변화를 desk로 확인했다. 90%에서 새로고침한 뒤 새 탭을 열고 원래 탭으로 돌아왔을 때 90%가 유지되고 새 탭은 100%인 것도 확인했다. 검증용 새 탭은 닫고 원래 탭을 100%로 복원해 새 실행본을 계속 실행 중으로 두었다. Windows/macOS의 이번 변경은 코드 확인과 공통 키 판정 테스트 범위이며 실제 실행 검증은 Linux에서 수행했다.


### 앱 전체 배율 저장으로 변경

2026-11-01 사용자 요청에 따라 위 탭별 배율 동작을 앱 전체 배율로 변경했다. 앱 상태에 배율 하나만 두고 키 입력을 직렬 작업 큐에서 처리한다. 설정 파일 저장이 성공하면 메모리와 열린 모든 콘텐츠 웹뷰에 반영하며, 새 콘텐츠 웹뷰는 표시 전에 저장된 배율을 적용한다. 탭 선택·복제·복구·딥링크에도 별도 배율을 만들지 않는다. Linux/macOS/Windows 공통 경로다.

Rust 테스트 13개(파일 저장 후 다시 읽기·100% 복원 저장·범위 제한·잘못된 설정의 기본값 포함)와 Node 단축키 테스트 4개, Linux release 빌드 및 diff 공백 검사가 통과했다. 기존 Projects 실행본에 새 빌드를 설치하고 바로가기로 재실행했다. Linux/KDE Wayland에서 두 탭을 연 상태로 두 번째 탭에서 120%로 변경한 뒤 첫 번째 탭도 120%인 것과 세 번째 새 탭의 120% 적용을 실제 화면으로 확인했다. 앱을 완전히 종료한 뒤 같은 바로가기로 다시 실행해 120% 복원을 확인했다. 실행 프로세스의 경로와 바이너리 SHA-256도 빌드 결과와 일치했다. 마지막에는 검증 전 배율인 100%로 복원해 실행해 두었다. Windows/macOS는 공통 코드 경로 확인 범위이며 이번 실행 검증 대상은 Linux다.
