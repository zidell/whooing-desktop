(async function () {
  const invoke = window.__TAURI__.core.invoke;
  const keys = window.__WHOOING_TAB_KEYS__;
  const tabs = document.getElementById('tabs');
  const status = document.getElementById('status');
  let state = { tabs: [], active: '' };
  let draggedId = null;
  async function action(name, args = {}) {
    try {
      status.textContent = '';
      await invoke('tabs_action', { action: name === 'new' ? 'open' : name, ...args });
    } catch (error) { status.textContent = String(error); }
  }
  function clearDrop() {
    tabs.querySelectorAll('.drop-before,.drop-after').forEach(row => row.classList.remove('drop-before', 'drop-after'));
  }
  function beforeId(row, event) {
    const index = state.tabs.findIndex(tab => tab.id === row.dataset.id);
    const rect = row.getBoundingClientRect();
    return event.clientX < rect.left + rect.width / 2 ? row.dataset.id : state.tabs[index + 1]?.id || null;
  }
  function createRow(id) {
    const row = document.createElement('div');
    row.className = 'tab';
    row.dataset.id = id;
    row.draggable = true;
    const select = document.createElement('button');
    select.className = 'select';
    select.setAttribute('role', 'tab');
    select.onclick = () => action('select', { id });
    const close = document.createElement('button');
    close.className = 'close';
    close.textContent = '×';
    close.onclick = () => action('close', { id });
    row.append(select, close);
    row.onauxclick = event => {
      if (event.button === 1) { event.preventDefault(); action('close', { id }); }
    };
    row.oncontextmenu = event => {
      event.preventDefault();
      const rect = row.getBoundingClientRect();
      action('context', { id, x: event.clientX || rect.left + 12, y: event.clientY || rect.bottom });
    };
    row.onkeydown = event => {
      if (event.altKey && event.shiftKey && !event.ctrlKey && !event.metaKey && ['ArrowLeft','ArrowRight'].includes(event.key)) {
        event.preventDefault();
        event.stopPropagation();
        action(event.key === 'ArrowLeft' ? 'move-left' : 'move-right', { id });
      } else if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
        event.preventDefault();
        const rect = row.getBoundingClientRect();
        action('context', { id, x: rect.left + 12, y: rect.bottom });
      } else if (!event.ctrlKey && !event.metaKey && !event.altKey && ['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) {
        event.preventDefault();
        const index = state.tabs.findIndex(tab => tab.id === id);
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? state.tabs.length - 1 : (index + (event.key === 'ArrowLeft' ? -1 : 1) + state.tabs.length) % state.tabs.length;
        action('select', { id: state.tabs[next].id });
      }
    };
    row.ondragstart = event => {
      if (event.target.closest('.close')) { event.preventDefault(); return; }
      draggedId = id;
      event.dataTransfer.setData('text/plain', id);
      event.dataTransfer.effectAllowed = 'move';
      row.classList.add('dragging');
    };
    row.ondragover = event => {
      if (!draggedId) return;
      event.preventDefault();
      event.stopPropagation();
      clearDrop();
      row.classList.add(beforeId(row, event) === id ? 'drop-before' : 'drop-after');
      event.dataTransfer.dropEffect = 'move';
      const edge = tabs.getBoundingClientRect();
      if (event.clientX < edge.left + 24) tabs.scrollLeft -= 24;
      if (event.clientX > edge.right - 24) tabs.scrollLeft += 24;
    };
    row.ondrop = event => {
      if (!draggedId) return;
      event.preventDefault();
      event.stopPropagation();
      action('move', { id: draggedId, beforeId: beforeId(row, event) });
      draggedId = null;
      clearDrop();
    };
    row.ondragend = () => { draggedId = null; row.classList.remove('dragging'); clearDrop(); };
    return row;
  }
  function render(next) {
    const activeChanged = state.active !== next.active;
    state = next;
    document.documentElement.style.setProperty('--tab-bar-height', state.tab_bar_height + 'px');
    document.querySelector('header').hidden = !state.tab_bar_visible;
    // Keep existing DOM nodes so asynchronous title updates do not cancel drags
    // or discard keyboard focus on the tab strip.
    const rows = new Map([...tabs.children].map(row => [row.dataset.id, row]));
    for (const [index, tab] of state.tabs.entries()) {
      const row = rows.get(tab.id) || createRow(tab.id);
      rows.delete(tab.id);
      row.classList.toggle('active', tab.id === state.active);
      const select = row.querySelector('.select');
      select.setAttribute('aria-selected', String(tab.id === state.active));
      select.setAttribute('aria-label', (index + 1) + ': ' + tab.title);
      select.tabIndex = tab.id === state.active ? 0 : -1;
      select.textContent = tab.title;
      const digit = index < 8 ? index + 1 : index === state.tabs.length - 1 ? 9 : null;
      select.title = tab.title + (digit ? ' (' + (keys.mac ? 'Cmd' : 'Ctrl') + '+' + digit + ')' : '');
      row.querySelector('.close').setAttribute('aria-label', tab.title + ' 탭 닫기');
      if (tabs.children[index] !== row) tabs.insertBefore(row, tabs.children[index] || null);
      if (activeChanged && tab.id === state.active) row.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
    for (const row of rows.values()) row.remove();
    status.textContent = state.tabs.length ? '' : '후잉을 불러오는 중…';
  }
  tabs.ondragover = event => { if (draggedId) event.preventDefault(); };
  tabs.ondrop = event => {
    if (draggedId) { event.preventDefault(); action('move', { id: draggedId, beforeId: null }); draggedId = null; clearDrop(); }
  };
  tabs.addEventListener('wheel', event => {
    if (tabs.scrollWidth > tabs.clientWidth) { event.preventDefault(); tabs.scrollLeft += event.deltaY || event.deltaX; }
  }, { passive: false });
  const primary = keys.mac ? 'Cmd' : 'Ctrl';
  document.getElementById('new').title = '새 후잉 탭 (' + primary + '+T)';
  document.getElementById('new').onclick = () => action('open');
  document.addEventListener('keydown', event => {
    const name = keys.action(event);
    if (name) { event.preventDefault(); event.stopImmediatePropagation(); action(name); }
  }, true);
  try {
    await window.__TAURI__.event.listen('tabs-changed', event => render(event.payload));
    await window.__TAURI__.event.listen('tabs-error', event => { status.textContent = event.payload; });
    render(await invoke('tabs_snapshot'));
  } catch (error) { status.textContent = '탭을 준비하지 못했습니다: ' + error; }
})();
