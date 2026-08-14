(function () {
  'use strict';

  const toggle = document.getElementById('theme-toggle');
  if (toggle) {
    toggle.addEventListener('click', flipTheme);
  }

  function flipTheme() {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    try { localStorage.setItem('theme', next); } catch (e) {}
  }

  document.querySelectorAll('[data-copy]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      const text = btn.getAttribute('data-copy');
      if (!text) return;
      try {
        navigator.clipboard.writeText(text);
        const orig = btn.textContent;
        btn.textContent = 'copied';
        setTimeout(function () { btn.textContent = orig; }, 1200);
      } catch (e) {}
    });
  });

  document.querySelectorAll('[data-install]').forEach(function (root) {
    const tabs = root.querySelectorAll('[data-install-tab]');
    const panes = root.querySelectorAll('[data-install-pane]');
    tabs.forEach(function (tab) {
      tab.addEventListener('click', function () {
        const name = tab.getAttribute('data-install-tab');
        tabs.forEach(function (t) {
          t.setAttribute('aria-selected', t === tab ? 'true' : 'false');
        });
        panes.forEach(function (p) {
          if (p.getAttribute('data-install-pane') === name) p.removeAttribute('hidden');
          else p.setAttribute('hidden', '');
        });
      });
    });
  });

  const navToggle = document.getElementById('nav-toggle');
  const topbarNav = document.getElementById('topbar-nav');
  function setNavOpen(open) {
    if (!topbarNav || !navToggle) return;
    topbarNav.dataset.open = open ? 'true' : 'false';
    navToggle.setAttribute('aria-expanded', open ? 'true' : 'false');
  }
  if (navToggle && topbarNav) {
    navToggle.addEventListener('click', function () {
      setNavOpen(topbarNav.dataset.open !== 'true');
    });
    topbarNav.addEventListener('click', function (e) {
      if (e.target.tagName === 'A') setNavOpen(false);
    });
  }

  const modal = document.getElementById('help-modal');
  function openModal() { if (modal) { modal.dataset.open = 'true'; modal.setAttribute('aria-hidden', 'false'); } }
  function closeModal() { if (modal) { modal.dataset.open = 'false'; modal.setAttribute('aria-hidden', 'true'); } }
  if (modal) modal.addEventListener('click', function (e) { if (e.target === modal) closeModal(); });

  const lightbox = document.getElementById('gif-lightbox');
  const lightboxImg = lightbox && lightbox.querySelector('.lightbox__img');
  function closeLightbox() {
    if (!lightbox) return;
    lightbox.dataset.open = 'false';
    lightbox.setAttribute('aria-hidden', 'true');
    lightboxImg.src = '';
  }
  if (lightbox) {
    lightbox.addEventListener('click', closeLightbox);
    document.querySelectorAll('.gif').forEach(function (gif) {
      gif.addEventListener('click', function () {
        // Enlarge whichever variant the active theme is showing.
        const theme = document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
        const img = gif.querySelector('.gif--' + theme) || gif.querySelector('img');
        if (!img) return;
        lightboxImg.src = img.currentSrc || img.src;
        lightboxImg.alt = img.alt || '';
        lightbox.dataset.open = 'true';
        lightbox.setAttribute('aria-hidden', 'false');
      });
    });
  }

  document.addEventListener('keydown', function (e) {
    const tag = (e.target && e.target.tagName) || '';
    const typing = tag === 'INPUT' || tag === 'TEXTAREA' || e.target.isContentEditable;

    if (e.key === 'Escape') {
      closeModal();
      closeLightbox();
      closeSearch();
      setNavOpen(false);
      if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
      return;
    }
    if (typing) return;

    if (e.key === '/') {
      e.preventDefault();
      const s = document.getElementById('search');
      if (s) s.focus();
    } else if (e.key === '?') {
      e.preventDefault();
      openModal();
    } else if (e.key === 't') {
      e.preventDefault();
      flipTheme();
    }
  });

  const searchInput = document.getElementById('search');
  const searchResults = document.getElementById('search-results');
  let searchReady = false;
  let searchIndex = null;
  let pageByUrl = {};
  let focusIdx = -1;

  const FUSE_URL = 'https://cdn.jsdelivr.net/npm/fuse.js@7.0.0/dist/fuse.min.js';

  function loadScript(src) {
    return new Promise(function (resolve, reject) {
      const script = document.createElement('script');
      script.src = src;
      script.onload = resolve;
      script.onerror = reject;
      document.head.appendChild(script);
    });
  }

  function ensureSearchLoaded() {
    if (searchReady || !window.SEARCH_INDEX_URL) return Promise.resolve();
    // Zola writes the page index to window.searchIndex. The transform writes
    // the per-heading paragraphs to window.headingBodies. Both are same-origin.
    // fuse.js comes from a CDN. Section bodies are cosmetic, so they may fail.
    return Promise.all([
      loadScript(window.SEARCH_INDEX_URL),
      typeof Fuse === 'undefined' ? loadScript(FUSE_URL) : Promise.resolve(),
      window.SEARCH_BODIES_URL ? loadScript(window.SEARCH_BODIES_URL).catch(function () {}) : Promise.resolve(),
    ]).then(initIndex);
  }

  function initIndex() {
    if (!window.searchIndex || typeof Fuse === 'undefined') return;
    // Pages carry title/description/body; headings carry title only and link
    // straight to their anchor, so a query like "Tables" jumps to the section.
    window.searchIndex.forEach(function (page) { pageByUrl[page.url] = page; });
    const docs = window.searchIndex.concat(window.searchHeadings || []);
    searchIndex = new Fuse(docs, {
      keys: [
        { name: 'title', weight: 3 },
        { name: 'description', weight: 2 },
        { name: 'body', weight: 1 },
      ],
      ignoreLocation: true,
      threshold: 0.3,
      minMatchCharLength: 2,
    });
    searchReady = true;
  }

  function openSearch() { if (searchResults) searchResults.dataset.open = 'true'; }
  function closeSearch() { if (searchResults) { searchResults.dataset.open = 'false'; searchResults.innerHTML = ''; focusIdx = -1; } }

  if (searchInput && searchResults) {
    searchInput.addEventListener('focus', ensureSearchLoaded);
    searchInput.addEventListener('input', function () {
      const q = searchInput.value.trim();
      if (!q) { closeSearch(); return; }
      ensureSearchLoaded().then(function () {
        if (!searchReady) return;
        const hits = searchIndex.search(q).slice(0, 8);
        if (!hits.length) {
          searchResults.innerHTML = '<div class="sr-empty">no matches</div>';
          openSearch();
          return;
        }
        const bodies = window.headingBodies || {};
        searchResults.innerHTML = hits.map(function (h) {
          const doc = h.item;
          if (doc.body) return renderResult(doc.url, doc.title, doc.description, doc.body);
          // Heading hit: show its section's first paragraph and the page title.
          const page = pageByUrl[doc.url.split('#')[0]] || {};
          const body = bodies[(page.title || '') + '\n' + doc.title] || page.body || '';
          return renderResult(doc.url, doc.title, page.title, body);
        }).join('');
        focusIdx = -1;
        openSearch();
      });
    });

    searchInput.addEventListener('keydown', function (e) {
      const items = searchResults.querySelectorAll('a');
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

  function renderResult(url, title, context, body) {
    const snippet = body.replace(/\s+/g, ' ');
    const shown = snippet.length > 260 ? snippet.slice(0, 260) + '…' : snippet;
    return '<a href="' + url + '">' +
             '<span class="sr-title">' + escapeHtml(title || url) + '</span>' +
             (context ? '<span class="sr-section">' + escapeHtml(context) + '</span>' : '') +
             (shown ? '<div class="sr-snippet">' + escapeHtml(shown) + '</div>' : '') +
           '</a>';
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }
})();
