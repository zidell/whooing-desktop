# 데스크톱 멀티탭

Windows·macOS·Linux 모두 로컬 탭바 웹뷰와 콘텐츠용 child webview를 사용하는 공통 구현이다. 탭이 하나면 탭바와 해당 공간을 숨기고 두 개 이상이면 높이 32 논리 픽셀로 표시한다. Windows/macOS는 콘텐츠 좌표·높이를 조정하고 Linux는 GTK box packing과 shell 표시 여부를 조정한다. 탭 색상은 OS 라이트/다크 설정을 따른다.

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
