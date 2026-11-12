/**
 * Verbatim content script: page text/media detection, region selection,
 * clipboard bridging, and a Shadow-DOM result panel that never picks up the
 * host page's CSS.
 */

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface Settings {
  enabled: boolean;
  showFloating: boolean;
}

let panelHost: HTMLDivElement | null = null;
let panelRoot: ShadowRoot | null = null;
let panelTitle: HTMLElement | null = null;
let panelBody: HTMLElement | null = null;
let panelDot: HTMLElement | null = null;
let floatingHost: HTMLDivElement | null = null;

chrome.runtime.onMessage.addListener((message: any) => {
  if (message?.type === 'verbatim:show-panel') {
    showVerbatimPanel(String(message.title || 'Verbatim'), String(message.text || ''), message.tone || 'result');
  } else if (message?.type === 'verbatim:copy') {
    void copyText(String(message.text || ''));
  } else if (message?.type === 'verbatim:context') {
    void handleContextAction(message);
  }
});

void initialise();

async function initialise(): Promise<void> {
  const stored = await chrome.storage.sync.get({ enabled: true, showFloating: true });
  if (stored.enabled !== false && stored.showFloating !== false) ensureFloatingButton();
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== 'sync') return;
    const enabled = changes.enabled ? Boolean(changes.enabled.newValue) : true;
    const show = changes.showFloating ? Boolean(changes.showFloating.newValue) : true;
    if (enabled && show) ensureFloatingButton();
    else removeFloatingButton();
  });
}

async function handleContextAction(message: any): Promise<void> {
  showVerbatimPanel('Verbatim', 'Working locally…', 'loading');
  if (message.action === 'ocr-image') {
    const rect = findImageRect(String(message.srcUrl || ''));
    if (rect) {
      await sendRuntime({
        type: 'ocr-image',
        rect,
        dpr: window.devicePixelRatio || 1
      });
    } else {
      await selectRegion();
    }
  } else if (message.action === 'select-region') {
    await selectRegion();
  } else if (message.action === 'capture-text') {
    await sendRuntime({ type: 'capture-text', text: visibleText() });
  } else if (message.action === 'transcribe') {
    await sendRuntime({ type: 'transcribe-media', mediaUrl: detectMediaUrl() });
  }
}

function visibleText(): string {
  const title = document.title ? `${document.title}\n\n` : '';
  const body = document.body?.innerText || '';
  return (title + body).replace(/\n{3,}/g, '\n\n').trim().slice(0, 20000);
}

function detectMediaUrl(): string {
  const media = Array.from(document.querySelectorAll<HTMLMediaElement>('video, audio')).find(
    (element) => element.currentSrc || element.src
  );
  let source = media?.currentSrc || media?.src || '';
  if (!source) {
    const sourceElement = document.querySelector<HTMLSourceElement>('video source, audio source');
    source = sourceElement?.src || '';
  }
  if (source && !source.startsWith('blob:') && !source.startsWith('data:')) {
    try {
      return new URL(source, location.href).href;
    } catch {
      return source;
    }
  }
  if (/youtube\.com|youtu\.be|tiktok\.com|vimeo\.com|instagram\.com|twitter\.com|x\.com/i.test(location.hostname)) {
    return location.href;
  }
  return location.href;
}

function findImageRect(srcUrl: string): Rect | null {
  const normalize = (value: string) => {
    try {
      return new URL(value, location.href).href;
    } catch {
      return value;
    }
  };
  const needle = normalize(srcUrl);
  const candidates = Array.from(document.images).filter((image) => {
    const source = image.currentSrc || image.src;
    return source && (normalize(source) === needle || source.endsWith(srcUrl));
  });
  for (const image of candidates) {
    const rect = image.getBoundingClientRect();
    if (rect.width > 4 && rect.height > 4 && rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth) {
      return {
        x: Math.max(0, rect.left),
        y: Math.max(0, rect.top),
        w: Math.min(innerWidth, rect.right) - Math.max(0, rect.left),
        h: Math.min(innerHeight, rect.bottom) - Math.max(0, rect.top)
      };
    }
  }
  return null;
}

async function selectRegion(): Promise<void> {
  const host = document.createElement('div');
  host.style.cssText = 'position:fixed;inset:0;z-index:2147483647;';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = `
    <style>
      .overlay{position:fixed;inset:0;background:rgba(7,11,22,.34);cursor:crosshair;user-select:none}
      .box{position:fixed;border:1.5px solid #4ecca3;background:rgba(78,204,163,.12);box-shadow:0 0 0 9999px rgba(7,11,22,.28);display:none}
      .hint{position:fixed;left:50%;top:22px;transform:translateX(-50%);background:#16213e;border:1px solid rgba(78,204,163,.35);border-radius:8px;color:#eaeaea;font:11px system-ui,sans-serif;padding:8px 12px;box-shadow:0 10px 30px rgba(0,0,0,.4)}
      .hint b{color:#4ecca3}
    </style>
    <div class="overlay"></div>
    <div class="box"></div>
    <div class="hint"><b>Verbatim OCR</b> · drag to select a region · Esc to cancel</div>
  `;
  document.documentElement.appendChild(host);
  const overlay = root.querySelector<HTMLElement>('.overlay')!;
  const box = root.querySelector<HTMLElement>('.box')!;
  let start: { x: number; y: number } | null = null;
  let current: { x: number; y: number } | null = null;

  const cleanup = () => {
    window.removeEventListener('keydown', onKey, true);
    host.remove();
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      cleanup();
      showVerbatimPanel('Verbatim OCR', 'Region capture cancelled.', 'info');
    }
  };
  window.addEventListener('keydown', onKey, true);

  overlay.addEventListener('pointerdown', (event) => {
    start = { x: event.clientX, y: event.clientY };
    current = start;
    box.style.display = 'block';
    overlay.setPointerCapture(event.pointerId);
  });
  overlay.addEventListener('pointermove', (event) => {
    if (!start) return;
    current = { x: event.clientX, y: event.clientY };
    draw();
  });
  overlay.addEventListener('pointerup', () => {
    if (!start || !current) return;
    const rect = {
      x: Math.min(start.x, current.x),
      y: Math.min(start.y, current.y),
      w: Math.abs(current.x - start.x),
      h: Math.abs(current.y - start.y)
    };
    cleanup();
    if (rect.w < 5 || rect.h < 5) {
      showVerbatimPanel('Verbatim OCR', 'Selection was too small. Try again.', 'info');
      return;
    }
    showVerbatimPanel('Verbatim OCR', 'Reading the selected region locally…', 'loading');
    void sendRuntime({ type: 'ocr-region', rect, dpr: window.devicePixelRatio || 1 });
  });

  function draw() {
    if (!start || !current) return;
    const left = Math.min(start.x, current.x);
    const top = Math.min(start.y, current.y);
    box.style.left = `${left}px`;
    box.style.top = `${top}px`;
    box.style.width = `${Math.abs(current.x - start.x)}px`;
    box.style.height = `${Math.abs(current.y - start.y)}px`;
  }
}

function ensurePanel(): ShadowRoot {
  if (panelHost && panelRoot) return panelRoot;
  panelHost = document.createElement('div');
  panelHost.id = 'verbatim-panel-host';
  panelHost.style.cssText = 'position:fixed;top:16px;right:16px;z-index:2147483647;';
  panelRoot = panelHost.attachShadow({ mode: 'open' });
  panelRoot.innerHTML = `
    <style>
      :host{all:initial}
      .panel{width:min(390px,calc(100vw - 32px));max-height:min(500px,calc(100vh - 32px));display:flex;flex-direction:column;background:linear-gradient(165deg,#1a294a,#10182e);border:1px solid rgba(234,234,234,.14);border-radius:12px;box-shadow:0 24px 70px rgba(0,0,0,.55);font-family:Inter,'Segoe UI',system-ui,sans-serif;color:#eaeaea;overflow:hidden;animation:in .18s ease-out}
      header{display:flex;align-items:center;gap:8px;padding:11px 12px;border-bottom:1px solid rgba(234,234,234,.08)}
      .logo{width:22px;height:22px;border-radius:6px;background:#0f3460;border:1px solid rgba(78,204,163,.3);color:#4ecca3;display:flex;align-items:center;justify-content:center;font:700 9px ui-monospace,monospace;letter-spacing:.08em;flex:0 0 auto}
      .title{font-size:11px;font-weight:600;flex:1;color:#fff;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
      .dot{width:5px;height:5px;border-radius:50%;background:#4ecca3;box-shadow:0 0 7px rgba(78,204,163,.5);flex:0 0 auto}
      .dot.error{background:#e94560;box-shadow:0 0 7px rgba(233,69,96,.5)}
      .dot.info{background:#8c98ab;box-shadow:none}
      .close{background:transparent;border:0;color:#8c98ab;font-size:16px;line-height:1;cursor:pointer;padding:0 2px}
      .close:hover{color:#eaeaea}
      .body{padding:12px;font-size:11px;line-height:1.62;color:#d4dbe4;overflow-y:auto;white-space:pre-wrap;word-break:break-word;max-height:340px;min-height:42px}
      .body.loading{color:#8c98ab;font-style:italic;display:flex;align-items:center;gap:8px;min-height:56px}
      .spinner{width:12px;height:12px;border:1.5px solid rgba(78,204,163,.22);border-top-color:#4ecca3;border-radius:50%;animation:spin .8s linear infinite;flex:0 0 auto}
      footer{display:flex;align-items:center;gap:6px;padding:8px 10px;border-top:1px solid rgba(234,234,234,.08)}
      button.action{background:transparent;border:1px solid rgba(234,234,234,.13);border-radius:4px;color:#aeb8c6;font:9px Inter,system-ui,sans-serif;padding:4px 8px;cursor:pointer}
      button.action:hover{border-color:rgba(78,204,163,.4);color:#fff}
      .privacy{margin-left:auto;font:7px ui-monospace,monospace;letter-spacing:.07em;color:#626f84}
      @keyframes spin{to{transform:rotate(360deg)}}
      @keyframes in{from{opacity:0;transform:translateY(-5px)}}
    </style>
    <div class="panel">
      <header><div class="logo">V</div><div class="title">Verbatim</div><span class="dot"></span><button class="close" aria-label="Close">×</button></header>
      <div class="body"></div>
      <footer><button class="action copy">Copy</button><button class="action close-action">Close</button><span class="privacy">LOCAL ONLY</span></footer>
    </div>
  `;
  document.documentElement.appendChild(panelHost);
  panelTitle = panelRoot.querySelector('.title');
  panelBody = panelRoot.querySelector('.body');
  panelDot = panelRoot.querySelector('.dot');
  panelRoot.querySelector('.close')!.addEventListener('click', closePanel);
  panelRoot.querySelector('.close-action')!.addEventListener('click', closePanel);
  panelRoot.querySelector('.copy')!.addEventListener('click', () => {
    if (panelBody) void copyText(panelBody.textContent || '');
  });
  return panelRoot;
}

function showVerbatimPanel(title: string, text: string, tone: 'loading' | 'result' | 'error' | 'info'): void {
  ensurePanel();
  if (!panelRoot || !panelTitle || !panelBody || !panelDot) return;
  panelTitle.textContent = title;
  panelDot.className = 'dot' + (tone === 'error' ? ' error' : tone === 'info' ? ' info' : '');
  panelBody.className = 'body' + (tone === 'loading' ? ' loading' : '');
  if (tone === 'loading') {
    panelBody.innerHTML = '<span class="spinner"></span><span></span>';
    const label = panelBody.querySelector('span:last-child');
    if (label) label.textContent = text;
  } else {
    panelBody.textContent = text;
  }
  panelHost!.style.display = 'block';
}

function closePanel(): void {
  if (panelHost) panelHost.style.display = 'none';
}

function ensureFloatingButton(): void {
  if (floatingHost) return;
  floatingHost = document.createElement('div');
  floatingHost.id = 'verbatim-floating';
  floatingHost.style.cssText = 'position:fixed;right:18px;bottom:18px;z-index:2147483646;';
  const root = floatingHost.attachShadow({ mode: 'open' });
  root.innerHTML = `
    <style>
      :host{all:initial}
      button{width:42px;height:42px;border-radius:13px;background:linear-gradient(160deg,#1a294a,#0f1a33);border:1px solid rgba(78,204,163,.45);box-shadow:0 10px 30px rgba(0,0,0,.35),0 0 0 5px rgba(78,204,163,.03);color:#4ecca3;cursor:pointer;display:flex;align-items:center;justify-content:center;font:800 14px Inter,system-ui,sans-serif;letter-spacing:.04em;transition:transform .14s ease,border-color .14s ease}
      button:hover{transform:translateY(-1px);border-color:rgba(78,204,163,.8)}
      .badge{position:absolute;right:-2px;bottom:-2px;width:10px;height:10px;border-radius:50%;background:#4ecca3;border:2px solid #1a1a2e}
    </style>
    <button title="Verbatim: OCR this viewport" aria-label="Verbatim OCR viewport">V<span class="badge"></span></button>
  `;
  root.querySelector('button')!.addEventListener('click', () => {
    showVerbatimPanel('Verbatim OCR', 'Capturing this viewport locally…', 'loading');
    void sendRuntime({ type: 'capture-viewport' });
  });
  document.documentElement.appendChild(floatingHost);
}

function removeFloatingButton(): void {
  floatingHost?.remove();
  floatingHost = null;
}

async function sendRuntime(message: unknown): Promise<any> {
  try {
    return await chrome.runtime.sendMessage(message);
  } catch (error) {
    showVerbatimPanel('Verbatim', `The Verbatim extension worker is unavailable: ${String(error)}`, 'error');
    return null;
  }
}

async function copyText(text: string): Promise<void> {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = document.createElement('textarea');
    area.value = text;
    area.style.cssText = 'position:fixed;left:-9999px;top:0;';
    document.body.appendChild(area);
    area.select();
    document.execCommand('copy');
    area.remove();
  }
}

