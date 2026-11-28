// Connects the Ascension System UI to the desktop app:
// - loads your save from disk before the UI starts, and writes every change back
// - fetches your Brightspace feed directly (every 3 hours while the app is open)
(function () {
  'use strict';
  const T = window.__TAURI__;
  if (!T) return; // opened outside the desktop app (e.g. in a browser for testing)
  const invoke = T.core.invoke;
  const KEY = 'ascension-state';
  let hasFeed = false;

  const ready = (async () => {
    await window.AscensionEngineReady;
    try {
      const saved = await invoke('load_state');
      if (saved) {
        let local = null;
        try { local = JSON.parse(localStorage.getItem(KEY)); } catch (e) { /* none */ }
        const disk = JSON.parse(saved);
        if (!local || (disk.updatedAt || 0) >= (local.updatedAt || 0)) localStorage.setItem(KEY, saved);
      }
    } catch (e) {
      console.error('Could not read the save file', e);
    }
    try { hasFeed = (await invoke('get_settings')).has_feed; } catch (e) { /* keep false */ }
  })();

  // The UI saves to localStorage; mirror each save to the file on disk (batched).
  const setItem = Storage.prototype.setItem;
  let pending = null;
  let timer = null;
  function flush() {
    clearTimeout(timer);
    const json = pending;
    pending = null;
    if (json != null) invoke('save_state', { json }).catch((e) => console.error('Save failed', e));
  }
  Storage.prototype.setItem = function (k, v) {
    setItem.call(this, k, v);
    if (this === window.localStorage && k === KEY) {
      pending = v;
      clearTimeout(timer);
      timer = setTimeout(flush, 400);
    }
  };
  addEventListener('beforeunload', flush);
  addEventListener('pagehide', flush);

  window.AscensionDesktop = {
    hasFeed: () => hasFeed,
    setFeed: async (url) => { await invoke('set_feed', { url }); hasFeed = true; },
    fetchFeed: () => invoke('fetch_feed'),
  };

  window.AscensionBoot = (start) =>
    ready.then(start, start).then(() => {
      invoke('ui_ready', { engine: window.AscensionEngine ? window.AscensionEngine.version : null }).catch(() => {});
      const sync = () => window.AscensionApp && window.AscensionApp.bsSync(false);
      setTimeout(sync, 3000);
      setInterval(sync, 3 * 3600 * 1000);
    });
})();
