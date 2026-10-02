// Runs at document start in both the local startup page and the remote webview.
(function () {
  var isWhooing = location.protocol === 'https:' &&
    (location.hostname === 'whooing.com' || location.hostname.endsWith('.whooing.com'));
  var isLocal = location.hostname === 'localhost' || location.hostname === 'tauri.localhost';
  if (!isWhooing && !isLocal) return;

  var overlay = document.createElement('div');
  overlay.id = 'whooing-desktop-loading';
  // Keep the site's styles from affecting the loading indicator.
  overlay.style.cssText = 'position:fixed;inset:0;z-index:2147483647;background:#fafafa;';
  var shadow = overlay.attachShadow({ mode: 'closed' });
  shadow.innerHTML = `
    <style>
      :host { color: #454545; font-family: system-ui, sans-serif; }
      .loading { height: 100%; display: flex; align-items: center;
        justify-content: center; flex-direction: column; gap: 18px; }
      .spinner { width: 38px; height: 38px; border: 3px solid #e4e4e4;
        border-top-color: #57977b; border-radius: 50%; animation: spin .85s linear infinite; }
      p { margin: 0; font-size: 14px; }
      .slow { max-width: 320px; text-align: center; line-height: 1.6; color: #777; }
      [hidden] { display: none !important; }
      button { margin-top: 12px; padding: 8px 16px; border: 1px solid #ddd;
        border-radius: 6px; background: white; color: #454545; cursor: pointer; font: inherit; }
      @keyframes spin { to { transform: rotate(360deg); } }
      @media (prefers-reduced-motion: reduce) { .spinner { animation-duration: 2s; } }
    </style>
    <main class="loading" role="status" aria-live="polite">
      <div class="spinner" aria-hidden="true"></div>
      <p>후잉을 불러오는 중…</p>
      <div class="slow" hidden>
        <p>연결에 시간이 걸리고 있습니다.<br>인터넷 연결을 확인하거나 다시 시도해 주세요.</p>
        <button type="button">다시 시도</button>
      </div>
    </main>`;
  shadow.querySelector('button').addEventListener('click', function () { location.reload(); });
  var slowTimer = setTimeout(function () {
    shadow.querySelector('.slow').hidden = false;
  }, 15000);
  var observer = new MutationObserver(mount);
  function mount() {
    if (document.documentElement && !overlay.isConnected) {
      document.documentElement.appendChild(overlay);
      observer.disconnect();
    }
  }
  observer.observe(document, { childList: true });
  mount();

  if (isWhooing) {
    function finish() {
      observer.disconnect();
      clearTimeout(slowTimer);
      // Let the page paint after deferred scripts and DOMContentLoaded handlers.
      requestAnimationFrame(function () {
        requestAnimationFrame(function () { overlay.remove(); });
      });
    }
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', finish, { once: true });
    } else {
      finish();
    }
  }
})();
