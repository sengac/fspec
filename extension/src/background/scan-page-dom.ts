/**
 * Injected DOM scanning function for browser_scan_page.
 *
 * This function is passed to chrome.scripting.executeScript() and runs
 * in the page's ISOLATED world. It MUST be fully self-contained — no
 * closures over external state, no imports.
 *
 * Returns raw element data + metadata to the service worker for
 * ref assignment and tree formatting.
 *
 * Implemented by: LOCATE-004, LOCATE-007
 */

/** Result shape returned by the injected function */
export interface ScanResult {
  elements: Array<{
    tagName: string;
    role: string;
    name: string;
    selector: string;
    interactive: boolean;
    depth: number;
    attributes: Record<string, string>;
  }>;
  metadata: {
    url: string;
    title: string;
    viewportWidth: number;
    viewportHeight: number;
    totalElements: number;
  };
}

/**
 * The injected scanning function. Fully self-contained.
 *
 * @param interactiveMode - If true, only interactive elements get selectors
 * @param scope - Optional CSS selector to scope the scan root
 */
export function scanPageDOM(
  interactiveMode: boolean,
  scope?: string | null
): ScanResult {
  /* ── Constants (inlined — no closures allowed) ────────────── */

  const INTERACTABLE_SEL = [
    'a[href]',
    'button',
    'input',
    'textarea',
    'select',
    '[role="button"]',
    '[role="link"]',
    '[role="checkbox"]',
    '[role="radio"]',
    '[role="tab"]',
    '[role="menuitem"]',
    '[role="option"]',
    '[role="switch"]',
    '[role="textbox"]',
    '[role="combobox"]',
    '[role="searchbox"]',
    '[role="slider"]',
    '[role="spinbutton"]',
    '[contenteditable="true"]',
    '[contenteditable=""]',
    '[tabindex]',
    'summary',
    'details',
  ].join(',');

  const INPUT_TYPE_ROLES: Record<string, string> = {
    text: 'textbox',
    email: 'textbox',
    password: 'textbox',
    tel: 'textbox',
    url: 'textbox',
    search: 'searchbox',
    checkbox: 'checkbox',
    radio: 'radio',
    number: 'spinbutton',
    range: 'slider',
    submit: 'button',
    reset: 'button',
    button: 'button',
    image: 'button',
  };

  const TAG_ROLES: Record<string, string> = {
    BUTTON: 'button',
    TEXTAREA: 'textbox',
    SELECT: 'combobox',
    NAV: 'navigation',
    MAIN: 'main',
    ASIDE: 'complementary',
    FOOTER: 'contentinfo',
    HEADER: 'banner',
    SUMMARY: 'button',
    DETAILS: 'group',
  };

  const RELEVANT_ATTRS = [
    'type',
    'checked',
    'selected',
    'expanded',
    'pressed',
    'disabled',
    'required',
    'placeholder',
    'min',
    'max',
    'minlength',
    'maxlength',
    'step',
    'pattern',
    'accept',
    'multiple',
    'inputmode',
    'autocomplete',
    'aria-expanded',
    'aria-checked',
    'contenteditable',
  ];

  const FRAMEWORK_ID_RES = [/^r-/, /^ember\d/, /^react-/, /^:r[0-9]/];

  const COMPOUND_INPUT_SET = new Set([
    'date',
    'time',
    'datetime-local',
    'month',
    'week',
    'range',
    'number',
    'color',
    'file',
  ]);

  const DYNAMIC_CLASS_SET = new Set([
    'focus',
    'hover',
    'active',
    'selected',
    'disabled',
    'animation',
    'transition',
    'loading',
    'open',
    'closed',
    'expanded',
    'collapsed',
    'visible',
    'hidden',
    'pressed',
    'checked',
    'highlighted',
    'current',
    'entering',
    'leaving',
  ]);

  function filterDynClasses(classStr: string): string {
    return classStr
      .split(/\s+/)
      .filter(c => {
        if (!c) {
          return false;
        }
        const lower = c.toLowerCase();
        if (DYNAMIC_CLASS_SET.has(lower)) {
          return false;
        }
        for (const p of DYNAMIC_CLASS_SET) {
          if (lower.includes(p)) {
            return false;
          }
        }
        return true;
      })
      .sort()
      .join(' ');
  }

  /* ── Helper functions (all inlined) ───────────────────────── */

  function isDynId(id: string): boolean {
    for (const re of FRAMEWORK_ID_RES) {
      if (re.test(id)) {
        return true;
      }
    }
    const digitCount = (id.match(/\d/g) ?? []).length;
    if (id.length > 0 && digitCount / id.length > 0.3) {
      return true;
    }
    if (id.length > 8) {
      const hex = id.replace(/[^a-f0-9]/gi, '');
      if (hex.length > 8) {
        return true;
      }
    }
    return false;
  }

  function getRole(el: Element): string {
    const explicit = el.getAttribute('role');
    if (explicit) {
      return explicit;
    }
    const tag = el.tagName;
    const hm = /^H([1-6])$/.exec(tag);
    if (hm) {
      return 'heading';
    }
    if (tag === 'A' && el.hasAttribute('href')) {
      return 'link';
    }
    if (tag === 'INPUT') {
      const t = (el.getAttribute('type') ?? 'text').toLowerCase();
      return INPUT_TYPE_ROLES[t] ?? 'textbox';
    }
    if (tag === 'SECTION' && el.hasAttribute('aria-label')) {
      return 'region';
    }
    return TAG_ROLES[tag] ?? '';
  }

  function getName(el: Element): string {
    const al = el.getAttribute('aria-label');
    if (al) {
      return al.trim();
    }
    const lb = el.getAttribute('aria-labelledby');
    if (lb) {
      const ref = el.ownerDocument.getElementById(lb);
      if (ref) {
        const t = (ref.textContent ?? '').trim();
        if (t) {
          return t.length <= 80 ? t : t.slice(0, 80) + '...';
        }
      }
    }
    const ph = el.getAttribute('placeholder');
    if (ph) {
      return ph.trim();
    }
    if ('value' in el && typeof (el as HTMLInputElement).value === 'string') {
      const v = (el as HTMLInputElement).value.trim();
      if (v) {
        return v.length <= 80 ? v : v.slice(0, 80) + '...';
      }
    }
    const alt = el.getAttribute('alt');
    if (alt) {
      return alt.trim();
    }
    const title = el.getAttribute('title');
    if (title) {
      return title.trim();
    }
    const txt = (el.textContent ?? '').trim();
    return txt.length <= 80 ? txt : txt.slice(0, 80) + '...';
  }

  function isVis(el: Element): boolean {
    const h = el as HTMLElement;
    if (typeof h.checkVisibility === 'function') {
      if (
        !h.checkVisibility({
          opacityProperty: true,
          visibilityProperty: true,
        } as CheckVisibilityOptions)
      ) {
        return false;
      }
    } else {
      // Fallback: check computed style on element and all ancestors
      const s = getComputedStyle(h);
      if (
        s.display === 'none' ||
        s.visibility === 'hidden' ||
        s.opacity === '0'
      ) {
        return false;
      }
      let anc = h.parentElement;
      while (anc) {
        const ps = getComputedStyle(anc);
        if (ps.display === 'none') {
          return false;
        }
        anc = anc.parentElement;
      }
    }
    const r = el.getBoundingClientRect();
    if (
      (r.width !== 0 || r.height !== 0 || r.top !== 0 || r.left !== 0) &&
      (r.width === 0 || r.height === 0)
    ) {
      return false;
    }
    return true;
  }

  function isInteractive(el: Element): boolean {
    if (el.getAttribute('aria-disabled') === 'true') {
      return false;
    }
    if (el.closest('[aria-hidden="true"]')) {
      return false;
    }
    if (el.closest('[inert]')) {
      return false;
    }
    const s = getComputedStyle(el);
    if (s.pointerEvents === 'none') {
      return false;
    }

    // Label wrapper detection
    if (el.tagName === 'LABEL') {
      if (el.hasAttribute('for')) {
        return false;
      }
      return hasFormControl(el, 2);
    }

    if (el.matches(INTERACTABLE_SEL)) {
      return true;
    }
    if (s.cursor === 'pointer') {
      return true;
    }
    if (
      el.hasAttribute('onclick') ||
      el.hasAttribute('onmousedown') ||
      el.hasAttribute('onkeydown')
    ) {
      return true;
    }

    // Search element heuristic
    if (isSearchEl(el)) {
      return true;
    }

    // Icon-size heuristic
    if (isIconSize(el)) {
      return true;
    }

    return false;
  }

  function hasFormControl(el: Element, maxDepth: number): boolean {
    if (maxDepth <= 0) {
      return false;
    }
    for (const child of Array.from(el.children)) {
      if (child.matches('input, select, textarea')) {
        return true;
      }
      if (hasFormControl(child, maxDepth - 1)) {
        return true;
      }
    }
    return false;
  }

  const SEARCH_IND = new Set([
    'search',
    'magnify',
    'glass',
    'lookup',
    'find',
    'query',
    'search-icon',
    'search-btn',
    'search-button',
    'searchbox',
  ]);

  function isSearchEl(el: Element): boolean {
    const tag = el.tagName;
    if (tag !== 'DIV' && tag !== 'SPAN') {
      return false;
    }
    const cn =
      el.className && typeof el.className === 'string'
        ? el.className.toLowerCase()
        : '';
    const id = (el.id || '').toLowerCase();
    for (const ind of SEARCH_IND) {
      if (cn.includes(ind) || id.includes(ind)) {
        return true;
      }
    }
    for (const attr of Array.from(el.attributes)) {
      if (attr.name.startsWith('data-')) {
        const val = attr.value.toLowerCase();
        for (const ind of SEARCH_IND) {
          if (val.includes(ind)) {
            return true;
          }
        }
      }
    }
    return false;
  }

  function isIconSize(el: Element): boolean {
    const r = el.getBoundingClientRect();
    if (r.width < 10 || r.width > 50 || r.height < 10 || r.height > 50) {
      return false;
    }
    return !!(
      (el.className &&
        typeof el.className === 'string' &&
        el.className.length > 0) ||
      el.getAttribute('role') ||
      el.getAttribute('data-action') ||
      el.getAttribute('aria-label')
    );
  }

  const PROPAGATING_SEL = [
    'a',
    'button',
    'div[role="button"]',
    'div[role="combobox"]',
    'span[role="button"]',
    'span[role="combobox"]',
    'input[role="combobox"]',
  ];
  const PROPAGATING_JOINED = PROPAGATING_SEL.join(',');
  const CONTAINMENT_THRESH = 0.99;

  function isPropagatingParent(el: Element): boolean {
    if (el.matches(PROPAGATING_JOINED)) {
      return true;
    }
    // Label wrapping form controls claims the inner input
    if (
      el.tagName === 'LABEL' &&
      !el.hasAttribute('for') &&
      hasFormControl(el, 2)
    ) {
      return true;
    }
    // Compound HTML5 inputs claim their shadow DOM sub-components
    if (el.tagName === 'INPUT') {
      const t = (el.getAttribute('type') ?? '').toLowerCase();
      if (COMPOUND_INPUT_SET.has(t)) {
        return true;
      }
    }
    return false;
  }

  function rectContains(parentRect: DOMRect, childRect: DOMRect): boolean {
    const childArea = childRect.width * childRect.height;
    if (childArea === 0) {
      return true;
    }
    const overlapX = Math.max(
      0,
      Math.min(childRect.right, parentRect.right) -
        Math.max(childRect.left, parentRect.left)
    );
    const overlapY = Math.max(
      0,
      Math.min(childRect.bottom, parentRect.bottom) -
        Math.max(childRect.top, parentRect.top)
    );
    return (overlapX * overlapY) / childArea >= CONTAINMENT_THRESH;
  }

  function getAttrs(el: Element): Record<string, string> {
    const attrs: Record<string, string> = {};
    const hm = /^H([1-6])$/.exec(el.tagName);
    if (hm) {
      attrs.level = hm[1];
    }
    for (const a of RELEVANT_ATTRS) {
      if (el.hasAttribute(a)) {
        attrs[a] = el.getAttribute(a) ?? '';
      }
    }
    const h = el as HTMLInputElement;
    if (h.required && !attrs.required) {
      attrs.required = '';
    }
    if (h.checked && !attrs.checked) {
      attrs.checked = '';
    }
    return attrs;
  }

  function genSelector(el: Element): string {
    const tid = el.getAttribute('data-testid');
    if (tid) {
      return `[data-testid="${tid}"]`;
    }
    if (el.id && !isDynId(el.id)) {
      return `#${el.id}`;
    }
    // attribute combo
    const tag = el.tagName.toLowerCase();
    const parts: string[] = [tag];
    for (const a of ['type', 'name', 'role', 'aria-label']) {
      const v = el.getAttribute(a);
      if (v) {
        parts.push(`[${a}="${v}"]`);
      }
    }
    // Use filtered (stable) classes when class attr exists
    if (el.className && typeof el.className === 'string') {
      const stable = filterDynClasses(el.className);
      if (stable) {
        parts.push(`.${stable.split(' ').join('.')}`);
      }
    }
    if (parts.length > 1) {
      const sel = parts.join('');
      try {
        if (el.ownerDocument.querySelectorAll(sel).length === 1) {
          return sel;
        }
      } catch {
        /* invalid selector — fall through */
      }
    }
    // nth-child path
    const chain: string[] = [];
    let cur: Element | null = el;
    while (cur && cur !== el.ownerDocument.body) {
      const parent: Element | null = cur.parentElement;
      if (!parent) {
        break;
      }
      const idx = Array.from(parent.children).indexOf(cur) + 1;
      chain.unshift(`${cur.tagName.toLowerCase()}:nth-child(${idx})`);
      cur = parent;
    }
    return chain.length > 0 ? `body > ${chain.join(' > ')}` : tag;
  }

  function depthOf(el: Element, root: Element): number {
    let d = 0;
    let cur: Element | null = el.parentElement;
    while (cur && cur !== root) {
      d++;
      cur = cur.parentElement;
    }
    return d;
  }

  /* ── Main scan ────────────────────────────────────────────── */

  const root = scope ? document.querySelector(scope) : document.body;
  const empty: ScanResult = {
    elements: [],
    metadata: {
      url: location.href,
      title: document.title,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
      totalElements: 0,
    },
  };
  if (!root) {
    return empty;
  }

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, {
    acceptNode(node: Node) {
      const tag = (node as Element).tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'NOSCRIPT') {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  const elements: ScanResult['elements'] = [];
  let totalElements = 0;

  // Track which elements are excluded by bounding box propagation
  const claimedElements = new Set<Element>();

  // First pass: find propagating interactive parents
  const propagatingParents: Element[] = [];
  let n: Node | null = walker.currentNode;
  while (n) {
    const el = n as Element;
    if (isVis(el) && isInteractive(el) && isPropagatingParent(el)) {
      propagatingParents.push(el);
    }
    n = walker.nextNode();
  }

  // Mark children that are 99%+ contained by a propagating parent
  for (const parent of propagatingParents) {
    const parentRect = parent.getBoundingClientRect();
    if (parentRect.width === 0 && parentRect.height === 0) {
      // No layout (e.g. jsdom) — fall back to DOM containment
      const childWalker = document.createTreeWalker(
        parent,
        NodeFilter.SHOW_ELEMENT,
        null
      );
      let child: Node | null = childWalker.nextNode();
      while (child) {
        claimedElements.add(child as Element);
        child = childWalker.nextNode();
      }
    } else {
      const childWalker = document.createTreeWalker(
        parent,
        NodeFilter.SHOW_ELEMENT,
        null
      );
      let child: Node | null = childWalker.nextNode();
      while (child) {
        const childRect = (child as Element).getBoundingClientRect();
        if (rectContains(parentRect, childRect)) {
          claimedElements.add(child as Element);
        }
        child = childWalker.nextNode();
      }
    }
  }

  // Reset walker for second pass
  const walker2 = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, {
    acceptNode(node: Node) {
      const tag = (node as Element).tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'NOSCRIPT') {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  let node: Node | null = walker2.currentNode;
  while (node) {
    const el = node as Element;
    totalElements++;

    if (!isVis(el)) {
      node = walker2.nextNode();
      continue;
    }

    // Skip claimed children
    if (claimedElements.has(el)) {
      node = walker2.nextNode();
      continue;
    }

    const role = getRole(el);
    const inter = isInteractive(el);

    if (interactiveMode) {
      // In interactive mode: include interactive elements + structural context
      // IFRAME elements are always emitted so mergeFrameResults can splice
      // iframe scan results at the correct DOM position.
      if (el.tagName === 'IFRAME') {
        const iframeAttrs: Record<string, string> = {};
        const src = el.getAttribute('src');
        if (src) {
          iframeAttrs.src = src;
        }
        const name = el.getAttribute('name');
        if (name) {
          iframeAttrs.name = name;
        }
        elements.push({
          tagName: 'IFRAME',
          role: 'iframe',
          name: src ?? '',
          selector: '',
          interactive: false,
          depth: depthOf(el, root),
          attributes: iframeAttrs,
        });
      } else if (
        inter ||
        role === 'heading' ||
        role === 'navigation' ||
        role === 'main' ||
        role === 'region' ||
        role === 'banner' ||
        role === 'complementary' ||
        role === 'contentinfo' ||
        role === 'group'
      ) {
        elements.push({
          tagName: el.tagName,
          role,
          name: getName(el),
          selector: inter ? genSelector(el) : '',
          interactive: inter,
          depth: depthOf(el, root),
          attributes: inter || role === 'heading' ? getAttrs(el) : {},
        });
      }
    } else {
      // Non-interactive mode: include all visible elements, no interactive flag
      elements.push({
        tagName: el.tagName,
        role: role || el.tagName.toLowerCase(),
        name: getName(el),
        selector: '',
        interactive: false,
        depth: depthOf(el, root),
        attributes: {},
      });
    }

    node = walker2.nextNode();
  }

  return {
    elements,
    metadata: {
      url: location.href,
      title: document.title,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
      totalElements,
    },
  };
}
