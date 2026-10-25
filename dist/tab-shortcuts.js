(function () {
  window.close = function () { location.href = 'whooing-tab://close'; };
  document.addEventListener('keydown', function (e) {
    const action = window.__WHOOING_TAB_KEYS__.action(e);
    if (action) {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (action === 'reload') location.reload();
      else if (action === 'back') history.back();
      else if (action === 'forward') history.forward();
      else location.href = 'whooing-tab://' + action;
    } else if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      if (e.key === 'ArrowLeft') { e.preventDefault(); history.back(); }
      if (e.key === 'ArrowRight') { e.preventDefault(); history.forward(); }
    }
  }, true);
})();
