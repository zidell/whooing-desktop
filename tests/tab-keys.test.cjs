const test = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
function keys(platform) {
  const context = { navigator: { platform }, window: {} };
  vm.runInNewContext(fs.readFileSync('dist/tab-keys.js', 'utf8'), context);
  return (key, overrides = {}) => context.window.__WHOOING_TAB_KEYS__.action({ key, code: '', ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...overrides });
}
for (const platform of ['Win32', 'Linux x86_64', 'MacIntel']) {
  const action = keys(platform);
  const primary = platform === 'MacIntel' ? { metaKey: true } : { ctrlKey: true };
  test(platform + ': direct tab selection and basic utilities', () => {
    for (let digit = 1; digit <= 9; digit++) assert.equal(action(String(digit), primary), 'select-' + digit);
    assert.equal(action('0', primary), null);
    assert.equal(action('T', { ...primary, shiftKey: true }), 'reopen');
    assert.equal(action('t', primary), 'new');
    assert.equal(action('w', primary), 'close');
    assert.equal(action('D', { ...primary, shiftKey: true }), 'duplicate');
    assert.equal(action('r', primary), 'reload');
    assert.equal(action('Tab', { ctrlKey: true }), 'next');
    assert.equal(action('Tab', { ctrlKey: true, shiftKey: true }), 'previous');
    assert.equal(action('PageUp', primary), 'previous');
    assert.equal(action('PageDown', { ...primary, shiftKey: true }), 'move-right');
    assert.equal(action('1', { ...primary, altKey: true }), null);
    assert.equal(action('1'), null);
  });
}
test('Mac preserves OS app switching and distinguishes tab versus history brackets', () => {
  const action = keys('MacIntel');
  assert.equal(action('Tab', { metaKey: true }), null);
  assert.equal(action('Tab', { metaKey: true, shiftKey: true }), null);
  assert.equal(action('1', { ctrlKey: true }), null);
  assert.equal(action('{', { metaKey: true, shiftKey: true, code: 'BracketLeft' }), 'previous');
  assert.equal(action('}', { metaKey: true, shiftKey: true, code: 'BracketRight' }), 'next');
  assert.equal(action('[', { metaKey: true, code: 'BracketLeft' }), 'back');
  assert.equal(action(']', { metaKey: true, code: 'BracketRight' }), 'forward');
});
