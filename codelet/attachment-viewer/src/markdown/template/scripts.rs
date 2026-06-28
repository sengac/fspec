//! Client-side scripts for the viewer — port of
//! `src/server/templates/viewer-scripts.ts`.
//!
//! All three constants are static server-emitted JavaScript: the mermaid module
//! loader (theme follows the persisted `fspec-theme`), the Prism core +
//! autoloader script tags, and the interaction script that highlights code
//! blocks (with a language alias map), adds copy buttons + language badges,
//! wires the theme toggle, and the clamped font-size controls.

/// Mermaid loader `<script type="module">` — theme follows `fspec-theme`.
pub const HEAD_SCRIPTS: &str = r#"
  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
    const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
    const savedTheme = localStorage.getItem('fspec-theme');
    const isDark = savedTheme ? savedTheme === 'dark' : prefersDark;
    mermaid.initialize({
      startOnLoad: true,
      theme: isDark ? 'dark' : 'default',
      securityLevel: 'loose',
      fontFamily: 'monospace',
      flowchart: { useMaxWidth: true, htmlLabels: true, curve: 'basis' }
    });
    if (isDark) {
      document.documentElement.classList.add('dark-theme');
    } else {
      document.documentElement.classList.add('light-theme');
    }
    window.addEventListener('DOMContentLoaded', () => { mermaid.run(); });
  </script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-core.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>
"#;

/// Interaction script: highlight + copy/badge + theme toggle + font controls.
pub const INTERACTION_SCRIPT: &str = r#"
  <script>
    Prism.plugins.autoloader.languages_path =
      'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';

    const languageMap = {
      sh: 'bash', shell: 'bash', console: 'bash',
      js: 'javascript', ts: 'typescript', py: 'python',
      rb: 'ruby', yml: 'yaml'
    };

    function getSupportedLanguage(language) {
      if (!language || language === 'text') return 'plaintext';
      return languageMap[language.toLowerCase()] || language;
    }

    window.addEventListener('DOMContentLoaded', () => {
      document.querySelectorAll('pre.code-block').forEach((pre) => {
        const code = pre.querySelector('code');
        const rawLanguage = pre.getAttribute('data-language') || 'text';
        const language = getSupportedLanguage(rawLanguage);
        if (code) { code.className = 'language-' + language; }

        const copyButton = document.createElement('button');
        copyButton.className = 'copy-button';
        copyButton.textContent = 'Copy';
        copyButton.onclick = () => {
          navigator.clipboard.writeText(code ? code.textContent || '' : '');
          copyButton.textContent = 'Copied!';
          setTimeout(() => { copyButton.textContent = 'Copy'; }, 2000);
        };
        pre.appendChild(copyButton);

        const badge = document.createElement('div');
        badge.className = 'language-badge';
        badge.textContent = rawLanguage;
        pre.appendChild(badge);
      });

      Prism.highlightAll();

      const toggleButton = document.getElementById('theme-toggle');
      const themeIcon = document.getElementById('theme-icon');
      const startDark = document.documentElement.classList.contains('dark-theme');
      if (themeIcon) { themeIcon.textContent = startDark ? '\u{1F319}' : '\u{2600}\u{FE0F}'; }

      if (toggleButton) {
        toggleButton.onclick = () => {
          const currentlyDark = document.documentElement.classList.contains('dark-theme');
          if (currentlyDark) {
            document.documentElement.classList.remove('dark-theme');
            document.documentElement.classList.add('light-theme');
            if (themeIcon) { themeIcon.textContent = '\u{2600}\u{FE0F}'; }
            localStorage.setItem('fspec-theme', 'light');
          } else {
            document.documentElement.classList.remove('light-theme');
            document.documentElement.classList.add('dark-theme');
            if (themeIcon) { themeIcon.textContent = '\u{1F319}'; }
            localStorage.setItem('fspec-theme', 'dark');
          }
        };
      }

      const MIN_FONT_SIZE = 10;
      const MAX_FONT_SIZE = 24;
      const FONT_SIZE_STEP = 2;
      const DEFAULT_FONT_SIZE = 16;

      const increaseBtn = document.getElementById('font-size-increase');
      const decreaseBtn = document.getElementById('font-size-decrease');
      const display = document.getElementById('font-size-display');

      const saved = localStorage.getItem('fspec-base-font-size');
      let currentFontSize = saved ? parseInt(saved, 10) : DEFAULT_FONT_SIZE;

      function applyFontSize(size) {
        document.documentElement.style.setProperty('--base-font-size', size + 'px');
        document.documentElement.style.setProperty('--font-scale', (size / 16).toString());
        if (display) { display.textContent = size + 'px'; }
      }

      function updateButtonStates() {
        if (decreaseBtn) { decreaseBtn.disabled = currentFontSize <= MIN_FONT_SIZE; }
        if (increaseBtn) { increaseBtn.disabled = currentFontSize >= MAX_FONT_SIZE; }
      }

      applyFontSize(currentFontSize);
      updateButtonStates();

      if (increaseBtn) {
        increaseBtn.onclick = () => {
          currentFontSize = Math.min(currentFontSize + FONT_SIZE_STEP, MAX_FONT_SIZE);
          applyFontSize(currentFontSize);
          localStorage.setItem('fspec-base-font-size', currentFontSize.toString());
          updateButtonStates();
        };
      }

      if (decreaseBtn) {
        decreaseBtn.onclick = () => {
          currentFontSize = Math.max(currentFontSize - FONT_SIZE_STEP, MIN_FONT_SIZE);
          applyFontSize(currentFontSize);
          localStorage.setItem('fspec-base-font-size', currentFontSize.toString());
          updateButtonStates();
        };
      }
    });
  </script>
"#;
