(function () {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  window.__WHOOING_TAB_KEYS__ = {
    mac,
    action(event) {
      if (event.altKey) return null;
      const primary = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
      const key = event.key.toLowerCase();
      if (event.ctrlKey && !event.metaKey && key === 'tab') return event.shiftKey ? 'previous' : 'next';
      if (!primary) return null;
      if (!event.shiftKey && /^[1-9]$/.test(key)) return 'select-' + key;
      if (key === 't') return event.shiftKey ? 'reopen' : 'new';
      if (key === 'w' && !event.shiftKey) return 'close';
      if (key === 'd' && event.shiftKey) return 'duplicate';
      if (key === 'r') return 'reload';
      if (key === 'pageup') return event.shiftKey ? 'move-left' : 'previous';
      if (key === 'pagedown') return event.shiftKey ? 'move-right' : 'next';
      if (mac && event.code === 'BracketLeft') return event.shiftKey ? 'previous' : 'back';
      if (mac && event.code === 'BracketRight') return event.shiftKey ? 'next' : 'forward';
      return null;
    }
  };
})();
