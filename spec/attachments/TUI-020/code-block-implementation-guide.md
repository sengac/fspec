# Syntax-Highlighted Code Block Implementation Guide

**Source Reference**: MindStrike project (`~/projects/mindstrike`)
**Primary Files**:
- `src/components/MarkdownViewer.tsx`
- `src/chat/components/ChatMessage.tsx`

---

## Overview

This guide details how to implement syntax-highlighted code blocks in the attachment viewer, based on the MindStrike implementation pattern.

## Architecture: Client-Side Rendering

**All syntax highlighting happens in the browser** (client-side), not on the server.

### Why Client-Side?
- No server CPU overhead for syntax processing
- Instant visual updates without server round-trips
- Better browser performance with modern JS engines
- Easier to add interactive features (copy buttons, theme toggle)

---

## Dependencies Required

```json
{
  "react-syntax-highlighter": "^15.6.1",
  "marked": "^14.1.3",
  "dompurify": "^3.2.2"
}
```

**Note**: We already have `marked` and `dompurify` installed. Only need to add `react-syntax-highlighter`.

---

## Implementation Steps

### Step 1: Code Block Detection (Server-Side)

**Location**: `src/server/utils/markdown-renderer.ts`

```typescript
// Regex to detect code blocks
const codeRegex = /```([a-zA-Z0-9_+-]*)\n?([\s\S]*?)\n?```/g;

// Extract code blocks before markdown processing
const codeBlocks: Array<{language: string, code: string}> = [];
let match;

while ((match = codeRegex.exec(markdown)) !== null) {
  const language = match[1] || 'text';
  const code = match[2].trim();

  codeBlocks.push({ language, code });

  // Replace with placeholder that won't be processed by marked
  const placeholder = `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  markdown = markdown.replace(match[0], placeholder);
}
```

### Step 2: Markdown to HTML Conversion (Server-Side)

```typescript
import { marked } from 'marked';

// Process markdown (code blocks already extracted)
const html = marked.parse(markdown) as string;
```

### Step 3: Inject Code Block Placeholders (Server-Side)

```typescript
// Replace placeholders with special markers for client-side rendering
let finalHtml = html;
codeBlocks.forEach((block, index) => {
  const marker = `<pre class="code-block" data-language="${block.language}" data-index="${index}">
  <code>${escapeHtml(block.code)}</code>
</pre>`;

  finalHtml = finalHtml.replace(`__CODE_BLOCK_${index}__`, marker);
});

// Escape HTML to prevent XSS
function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#039;'
  };
  return text.replace(/[&<>"']/g, char => map[char] || char);
}
```

### Step 4: Client-Side Highlighting (HTML Template)

**Location**: `src/server/templates/viewer-template.ts`

Add this script to the HTML template:

```html
<!-- Syntax highlighting library (Prism via CDN) -->
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism-themes/1.9.0/prism-vsc-dark-plus.min.css" />
<script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-core.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>

<script>
  // Configure Prism autoloader
  Prism.plugins.autoloader.languages_path = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';

  // Language mapping (60+ aliases)
  const languageMap = {
    sh: 'bash',
    shell: 'bash',
    console: 'bash',
    js: 'javascript',
    ts: 'typescript',
    py: 'python',
    rb: 'ruby',
    yml: 'yaml',
    c: 'c',
    cpp: 'cpp',
    'c++': 'cpp',
    cs: 'csharp',
    'c#': 'csharp',
    go: 'go',
    golang: 'go',
    rs: 'rust',
    java: 'java',
    kt: 'kotlin',
    swift: 'swift',
    php: 'php',
    sql: 'sql',
    html: 'markup',
    xml: 'markup',
    css: 'css',
    scss: 'scss',
    sass: 'sass',
    less: 'less',
    json: 'json',
    md: 'markdown',
    tex: 'latex',
    r: 'r',
    matlab: 'matlab',
    lua: 'lua',
    perl: 'perl',
    diff: 'diff',
    patch: 'diff',
    // Add more as needed
  };

  function getSupportedLanguage(language) {
    if (!language || language === 'text') {
      return 'plaintext';
    }
    const lower = language.toLowerCase();
    return languageMap[lower] || language;
  }

  // Apply syntax highlighting to all code blocks
  window.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('pre.code-block').forEach((pre) => {
      const code = pre.querySelector('code');
      const rawLanguage = pre.getAttribute('data-language') || 'text';
      const language = getSupportedLanguage(rawLanguage);

      // Set Prism language class
      code.className = `language-${language}`;
      pre.className = `code-block language-${language}`;

      // Add copy button
      const copyButton = document.createElement('button');
      copyButton.className = 'copy-button';
      copyButton.innerHTML = '📋 Copy';
      copyButton.onclick = () => {
        navigator.clipboard.writeText(code.textContent || '');
        copyButton.innerHTML = '✓ Copied!';
        setTimeout(() => {
          copyButton.innerHTML = '📋 Copy';
        }, 2000);
      };
      pre.appendChild(copyButton);

      // Add language badge (if multi-line)
      if (code.textContent && code.textContent.includes('\\n')) {
        const badge = document.createElement('div');
        badge.className = 'language-badge';
        badge.textContent = rawLanguage;
        pre.appendChild(badge);
      }
    });

    // Trigger Prism highlighting
    Prism.highlightAll();
  });
</script>
```

### Step 5: CSS Styling (HTML Template)

Add to the `<style>` section:

```css
/* Code block container */
pre.code-block {
  position: relative;
  background-color: #1e1e1e;
  border: 1px solid #404040;
  border-radius: 0.5rem;
  padding: 1rem;
  overflow-x: auto;
  margin: 1.5rem 0;
  font-family: 'Courier New', Courier, monospace;
}

pre.code-block code {
  background-color: transparent;
  padding: 0;
  font-size: 0.875rem;
  line-height: 1.5;
  color: #cccccc;
}

/* Copy button */
.copy-button {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  background-color: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #ffffff;
  padding: 0.25rem 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s, background-color 0.2s;
}

pre.code-block:hover .copy-button {
  opacity: 1;
}

.copy-button:hover {
  background-color: rgba(255, 255, 255, 0.2);
}

/* Language badge */
.language-badge {
  position: absolute;
  bottom: 0.5rem;
  right: 0.5rem;
  background-color: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  color: #ffffff;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  text-transform: uppercase;
}

/* Inline code */
code {
  font-family: 'Courier New', Courier, monospace;
  background-color: #2d2d30;
  color: #ffffff;
  padding: 0.125rem 0.25rem;
  border-radius: 0.25rem;
  font-size: 0.875em;
}
```

---

## Theme Integration (Dark/Light)

### Detect OS Theme Preference

```javascript
// Check if user prefers dark mode
const prefersDark = window.matchMedia &&
  window.matchMedia('(prefers-color-scheme: dark)').matches;

// Load saved preference from localStorage
const savedTheme = localStorage.getItem('fspec-theme');
const isDark = savedTheme ? savedTheme === 'dark' : prefersDark;

// Apply theme
if (isDark) {
  document.body.classList.add('dark-theme');
} else {
  document.body.classList.add('light-theme');
}
```

### Theme Toggle Button

```html
<button id="theme-toggle" class="theme-toggle">
  <span id="theme-icon">🌙</span>
</button>

<script>
  const toggleButton = document.getElementById('theme-toggle');
  const themeIcon = document.getElementById('theme-icon');

  toggleButton.onclick = () => {
    const isDark = document.body.classList.contains('dark-theme');

    if (isDark) {
      // Switch to light
      document.body.classList.remove('dark-theme');
      document.body.classList.add('light-theme');
      themeIcon.textContent = '☀️';
      localStorage.setItem('fspec-theme', 'light');
    } else {
      // Switch to dark
      document.body.classList.remove('light-theme');
      document.body.classList.add('dark-theme');
      themeIcon.textContent = '🌙';
      localStorage.setItem('fspec-theme', 'dark');
    }

    // Re-run Prism highlighting with new theme
    Prism.highlightAll();
  };
</script>
```

### CSS for Themes

```css
/* Light theme overrides */
body.light-theme {
  background-color: #ffffff;
  color: #1e1e1e;
}

body.light-theme pre.code-block {
  background-color: #f5f5f5;
  border-color: #d0d0d0;
}

body.light-theme code {
  background-color: #e5e5e5;
  color: #1e1e1e;
}

/* Use light Prism theme */
body.light-theme pre.code-block code {
  /* Load prism-tomorrow.min.css for light theme */
}

/* Theme toggle button */
.theme-toggle {
  position: fixed;
  top: 1rem;
  right: 1rem;
  background-color: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  padding: 0.5rem;
  border-radius: 0.5rem;
  cursor: pointer;
  font-size: 1.25rem;
  z-index: 1000;
}
```

---

## Language Support

MindStrike supports **60+ language aliases**. Key languages:

- **Web**: javascript, typescript, html, css, scss, json
- **Systems**: c, cpp, rust, go, java, kotlin, swift
- **Scripting**: python, ruby, php, perl, lua, bash
- **Data**: sql, yaml, toml, xml
- **Other**: diff, markdown, latex, r, matlab

Prism autoloader will fetch language grammars on-demand.

---

## Security Considerations

1. **HTML Escaping**: All code content MUST be escaped before insertion
2. **XSS Prevention**: Never use `dangerouslySetInnerHTML` for code blocks
3. **Clipboard Safety**: Use modern Clipboard API (navigator.clipboard)
4. **Content-Type Headers**: Ensure server sends `text/html; charset=utf-8`

---

## Testing Checklist

- [ ] Markdown with inline code (`code`)
- [ ] Multi-line code blocks with language hints
- [ ] Code blocks without language hints (default to plaintext)
- [ ] Copy button works (clipboard API)
- [ ] Language badge displays correctly
- [ ] Theme toggle switches between light/dark
- [ ] Theme preference persists in localStorage
- [ ] OS theme preference detected on first load
- [ ] Special characters escaped properly (< > & " ')
- [ ] Long lines wrap or scroll horizontally
- [ ] All supported languages render correctly

---

## References

- **MindStrike Source**: `~/projects/mindstrike/src/components/MarkdownViewer.tsx` (lines 17-407)
- **Prism Documentation**: https://prismjs.com/
- **Marked Documentation**: https://marked.js.org/
- **VSC Dark Plus Theme**: https://github.com/react-syntax-highlighter/react-syntax-highlighter

---

## Summary

1. **Server-side**: Extract code blocks, process markdown, inject placeholders
2. **Client-side**: Detect code blocks, map languages, apply Prism highlighting
3. **UI**: Add copy buttons, language badges, theme toggle
4. **Theme**: Detect OS preference, allow manual toggle, persist in localStorage
5. **Security**: Escape all code content, use safe clipboard API
