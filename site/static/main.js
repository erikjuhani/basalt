(function () {
  'use strict';

  // ── theme toggle ──────────────────────────────────────────────────────────
  var toggle = document.getElementById('theme-toggle');
  if (toggle) {
    toggle.addEventListener('click', flipTheme);
  }

  function flipTheme() {
    var next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    try { localStorage.setItem('theme', next); } catch (e) {}
  }

  // ── install-row copy buttons ──────────────────────────────────────────────
  document.querySelectorAll('[data-copy]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      var text = btn.getAttribute('data-copy');
      if (!text) return;
      try {
        navigator.clipboard.writeText(text);
        var orig = btn.textContent;
        btn.textContent = 'copied';
        setTimeout(function () { btn.textContent = orig; }, 1200);
      } catch (e) {}
    });
  });

  // ── help modal ────────────────────────────────────────────────────────────
  var modal = document.getElementById('help-modal');
  function openModal() { if (modal) { modal.dataset.open = 'true'; modal.setAttribute('aria-hidden', 'false'); } }
  function closeModal() { if (modal) { modal.dataset.open = 'false'; modal.setAttribute('aria-hidden', 'true'); } }
  if (modal) modal.addEventListener('click', function (e) { if (e.target === modal) closeModal(); });

  // ── keyboard shortcuts ────────────────────────────────────────────────────
  document.addEventListener('keydown', function (e) {
    var tag = (e.target && e.target.tagName) || '';
    var typing = tag === 'INPUT' || tag === 'TEXTAREA' || e.target.isContentEditable;

    if (e.key === 'Escape') {
      closeModal();
      closeSearch();
      if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
      return;
    }
    if (typing) return;

    if (e.key === '/') {
      e.preventDefault();
      var s = document.getElementById('search');
      if (s) s.focus();
    } else if (e.key === '?') {
      e.preventDefault();
      openModal();
    } else if (e.key === 't') {
      e.preventDefault();
      flipTheme();
    }
  });

  // ── search ────────────────────────────────────────────────────────────────
  var searchInput = document.getElementById('search');
  var searchResults = document.getElementById('search-results');
  var searchReady = false;
  var searchIndex = null;
  var searchData = null;
  var focusIdx = -1;

  function ensureSearchLoaded() {
    if (searchReady || !window.SEARCH_INDEX_URL) return Promise.resolve();
    return new Promise(function (resolve) {
      var s = document.createElement('script');
      s.src = window.SEARCH_INDEX_URL;
      s.onload = function () {
        if (typeof elasticlunr === 'undefined' || !window.searchIndex) {
          // Zola serializes the index to window.searchIndex; if elasticlunr isn't
          // loaded we fetch it from a CDN before building the index.
          var e = document.createElement('script');
          e.src = 'https://cdn.jsdelivr.net/npm/elasticlunr@0.9.5/elasticlunr.min.js';
          e.onload = function () { initIndex(); resolve(); };
          document.head.appendChild(e);
        } else {
          initIndex();
          resolve();
        }
      };
      document.head.appendChild(s);
    });
  }

  function initIndex() {
    if (!window.searchIndex || typeof elasticlunr === 'undefined') return;
    searchIndex = elasticlunr.Index.load(window.searchIndex);
    searchData = window.searchIndex.documentStore.docs;
    searchReady = true;
  }

  function openSearch() { if (searchResults) searchResults.dataset.open = 'true'; }
  function closeSearch() { if (searchResults) { searchResults.dataset.open = 'false'; searchResults.innerHTML = ''; focusIdx = -1; } }

  if (searchInput && searchResults) {
    searchInput.addEventListener('focus', ensureSearchLoaded);
    searchInput.addEventListener('input', function () {
      var q = searchInput.value.trim();
      if (!q) { closeSearch(); return; }
      ensureSearchLoaded().then(function () {
        if (!searchReady) return;
        var hits = searchIndex.search(q, { bool: 'AND', expand: true }).slice(0, 8);
        if (!hits.length) {
          searchResults.innerHTML = '<div class="sr-empty">no matches</div>';
          openSearch();
          return;
        }
        searchResults.innerHTML = hits.map(function (h) {
          var doc = searchData[h.ref];
          var snippet = (doc.body || '').replace(/\s+/g, ' ').slice(0, 140);
          return '<a href="' + h.ref + '">' +
                   '<span class="sr-title">' + escapeHtml(doc.title || h.ref) + '</span>' +
                   (doc.description ? '<span class="sr-section">' + escapeHtml(doc.description) + '</span>' : '') +
                   '<div class="sr-snippet">' + escapeHtml(snippet) + '…</div>' +
                 '</a>';
        }).join('');
        focusIdx = -1;
        openSearch();
      });
    });

    searchInput.addEventListener('keydown', function (e) {
      var items = searchResults.querySelectorAll('a');
      if (!items.length) return;
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        focusIdx = Math.min(focusIdx + 1, items.length - 1);
        highlight(items);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        focusIdx = Math.max(focusIdx - 1, -1);
        highlight(items);
      } else if (e.key === 'Enter' && focusIdx >= 0) {
        e.preventDefault();
        items[focusIdx].click();
      }
    });

    document.addEventListener('click', function (e) {
      if (!searchResults.contains(e.target) && e.target !== searchInput) closeSearch();
    });
  }

  function highlight(items) {
    items.forEach(function (a, i) { a.dataset.focus = i === focusIdx ? 'true' : 'false'; });
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }
})();
