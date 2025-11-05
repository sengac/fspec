/**
 * Viewer Scripts - Client-side JavaScript for markdown viewer
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

/**
 * Generates mermaid initialization script.
 *
 * @returns Mermaid script tag with initialization
 */
export function getMermaidScript(): string {
  return `
  <!-- Mermaid library -->
  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

    // Detect theme preference
    const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark').matches;
    const savedTheme = localStorage.getItem('fspec-theme');
    const isDark = savedTheme ? savedTheme === 'dark' : prefersDark;

    // Initialize mermaid with detected theme
    mermaid.initialize({
      startOnLoad: true,
      theme: isDark ? 'dark' : 'default',
      securityLevel: 'loose',
      fontFamily: 'monospace',
      flowchart: {
        useMaxWidth: true,
        htmlLabels: true,
        curve: 'basis'
      }
    });

    // Apply theme class
    if (isDark) {
      document.documentElement.classList.add('dark-theme');
    } else {
      document.documentElement.classList.add('light-theme');
    }

    // Re-render diagrams after DOM is ready
    window.addEventListener('DOMContentLoaded', () => {
      mermaid.run();
    });
  </script>
  `;
}

/**
 * Generates Prism syntax highlighting scripts.
 *
 * @returns Prism script tags
 */
export function getPrismScripts(): string {
  return `
  <!-- Prism JS for syntax highlighting -->
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-core.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>
  `;
}

/**
 * Generates client-side interaction script for code blocks and theme toggle.
 *
 * @returns Client-side script for UI interactions
 */
export function getInteractionScript(): string {
  return `
  <script>
    // Configure Prism autoloader
    Prism.plugins.autoloader.languages_path = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';

    // Language mapping for aliases
    const languageMap = {
      sh: 'bash', shell: 'bash', console: 'bash',
      js: 'javascript', ts: 'typescript', py: 'python',
      rb: 'ruby', yml: 'yaml'
    };

    function getSupportedLanguage(language) {
      if (!language || language === 'text') return 'plaintext';
      return languageMap[language.toLowerCase()] || language;
    }

    // Apply syntax highlighting and add UI features
    window.addEventListener('DOMContentLoaded', () => {
      document.querySelectorAll('pre.code-block').forEach((pre) => {
        const code = pre.querySelector('code');
        const rawLanguage = pre.getAttribute('data-language') || 'text';
        const language = getSupportedLanguage(rawLanguage);

        // Set Prism language class
        code.className = \`language-\${language}\`;

        // Add copy button
        const copyButton = document.createElement('button');
        copyButton.className = 'copy-button';
        copyButton.textContent = 'Copy';
        copyButton.onclick = () => {
          navigator.clipboard.writeText(code.textContent || '');
          copyButton.textContent = 'Copied!';
          setTimeout(() => { copyButton.textContent = 'Copy'; }, 2000);
        };
        pre.appendChild(copyButton);

        // Add language badge
        const badge = document.createElement('div');
        badge.className = 'language-badge';
        badge.textContent = rawLanguage;
        pre.appendChild(badge);
      });

      // Trigger Prism highlighting
      Prism.highlightAll();

      // Theme toggle functionality
      const toggleButton = document.getElementById('theme-toggle');
      const themeIcon = document.getElementById('theme-icon');
      const isDark = document.documentElement.classList.contains('dark-theme');

      themeIcon.textContent = isDark ? '🌙' : '☀️';

      toggleButton.onclick = () => {
        const currentlyDark = document.documentElement.classList.contains('dark-theme');

        if (currentlyDark) {
          document.documentElement.classList.remove('dark-theme');
          document.documentElement.classList.add('light-theme');
          themeIcon.textContent = '☀️';
          localStorage.setItem('fspec-theme', 'light');
        } else {
          document.documentElement.classList.remove('light-theme');
          document.documentElement.classList.add('dark-theme');
          themeIcon.textContent = '🌙';
          localStorage.setItem('fspec-theme', 'dark');
        }
      };

      // Font size controls functionality
      const MIN_FONT_SIZE = 10;
      const MAX_FONT_SIZE = 24;
      const FONT_SIZE_STEP = 2;
      const DEFAULT_FONT_SIZE = 16;

      const fontSizeIncreaseBtn = document.getElementById('font-size-increase');
      const fontSizeDecreaseBtn = document.getElementById('font-size-decrease');
      const fontSizeDisplay = document.getElementById('font-size-display');

      // Load saved font size from localStorage or use default
      const savedFontSize = localStorage.getItem('fspec-base-font-size');
      let currentFontSize = savedFontSize ? parseInt(savedFontSize, 10) : DEFAULT_FONT_SIZE;

      // Apply initial font size
      document.documentElement.style.setProperty('--base-font-size', currentFontSize + 'px');
      document.documentElement.style.setProperty('--font-scale', (currentFontSize / 16).toString());
      fontSizeDisplay.textContent = currentFontSize + 'px';

      // Update button states based on current font size
      function updateButtonStates() {
        fontSizeDecreaseBtn.disabled = currentFontSize <= MIN_FONT_SIZE;
        fontSizeIncreaseBtn.disabled = currentFontSize >= MAX_FONT_SIZE;
      }

      updateButtonStates();

      // Increase font size
      fontSizeIncreaseBtn.onclick = () => {
        const newSize = Math.min(currentFontSize + FONT_SIZE_STEP, MAX_FONT_SIZE);
        currentFontSize = newSize;
        document.documentElement.style.setProperty('--base-font-size', newSize + 'px');
        document.documentElement.style.setProperty('--font-scale', (newSize / 16).toString());
        fontSizeDisplay.textContent = newSize + 'px';
        localStorage.setItem('fspec-base-font-size', newSize.toString());
        updateButtonStates();
      };

      // Decrease font size
      fontSizeDecreaseBtn.onclick = () => {
        const newSize = Math.max(currentFontSize - FONT_SIZE_STEP, MIN_FONT_SIZE);
        currentFontSize = newSize;
        document.documentElement.style.setProperty('--base-font-size', newSize + 'px');
        document.documentElement.style.setProperty('--font-scale', (newSize / 16).toString());
        fontSizeDisplay.textContent = newSize + 'px';
        localStorage.setItem('fspec-base-font-size', newSize.toString());
        updateButtonStates();
      };
    });
  </script>
  `;
}
