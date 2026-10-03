export const MAX_CLIPBOARD_BYTES = 4 * 1024 * 1024;
const encoder = new TextEncoder();

function checkedPayload(payload) {
  const html = typeof payload?.html === 'string' ? payload.html : '';
  const plainText = typeof payload?.plainText === 'string' ? payload.plainText : '';
  const sourceUrl = typeof payload?.sourceUrl === 'string' ? payload.sourceUrl.trim() : '';
  const providerId = payload?.providerId;
  if (providerId != null && typeof providerId !== 'string') throw new Error('Choose a supported recipe provider or Automatic.');
  if (encoder.encode(html).length + encoder.encode(plainText).length > MAX_CLIPBOARD_BYTES) {
    throw new Error('Clipboard content exceeds the 4 MiB limit. Copy a smaller recipe selection.');
  }
  if (!html.trim() && !plainText.trim()) throw new Error('Paste a recipe before previewing.');
  if (sourceUrl) {
    let url;
    try { url = new URL(sourceUrl); } catch { throw new Error('Enter a valid HTTP or HTTPS recipe URL.'); }
    if (!['https:', 'http:'].includes(url.protocol)) throw new Error('Enter a valid HTTP or HTTPS recipe URL.');
  }
  return { html: payload?.html ?? '', plainText: payload?.plainText ?? '', sourceUrl: sourceUrl || null,
    ...(providerId ? {providerId} : {}) };
}

export async function parseClipboardPayload(payload, parser) {
  return parser(checkedPayload(payload));
}

// The worker keeps parsing untrusted, potentially large clipboard input off the UI thread.
export function mountClipboardImport(element, parserUrl, send) {
  const textarea = element.matches?.('textarea') ? element : element.querySelector('textarea');
  if (!textarea) throw new Error('Recipe import requires a textarea.');
  let html = '';
  let sourceUrl = '';
  let providerId = '';
  let worker = null;
  let workerUrl = null;
  let timer = null;
  let generation = 0;
  let disposed = false;
  const stop = () => {
    generation++;
    if (timer !== null) clearTimeout(timer);
    timer = null;
    worker?.terminate();
    worker = null;
    if (workerUrl !== null) URL.revokeObjectURL(workerUrl);
    workerUrl = null;
  };
  const captured = () => send({ type: 'captured', html: Boolean(html),
    htmlBytes: encoder.encode(html).length, textBytes: encoder.encode(textarea.value).length,
    plainText: textarea.value });
  const paste = event => {
    if (!event.clipboardData) return;
    event.preventDefault();
    stop();
    html = event.clipboardData.getData('text/html');
    textarea.value = event.clipboardData.getData('text/plain');
    captured();
  };
  const input = () => { stop(); html = ''; captured(); };
  const sourceChange = event => {
    stop();
    sourceUrl = event.detail?.sourceUrl ?? sourceUrl;
    providerId = event.detail?.providerId ?? providerId;
    captured();
  };
  const preview = event => {
    stop();
    sourceUrl = event.detail?.sourceUrl ?? sourceUrl;
    providerId = event.detail?.providerId ?? providerId;
    let payload;
    try { payload = checkedPayload({ html, plainText: textarea.value, sourceUrl, providerId }); }
    catch (error) { send({type: 'error', message: error.message}); return; }
    const token = generation;
    try {
      const moduleUrl = new URL(parserUrl, document.baseURI).href;
      const source = `import { parseRecipeClipboard } from ${JSON.stringify(moduleUrl)};
self.onmessage = ({data}) => {
  try { self.postMessage({type:'result',candidate:parseRecipeClipboard(data)}); }
  catch(error) { self.postMessage({type:'error',message:error.message || 'Recipe parsing failed.'}); }
};`;
      workerUrl = URL.createObjectURL(new Blob([source], {type: 'text/javascript'}));
      worker = new Worker(workerUrl, {type: 'module'});
      worker.onmessage = ({data}) => {
        if (disposed || generation !== token) return;
        stop();
        send(data);
      };
      worker.onerror = () => {
        if (disposed || generation !== token) return;
        stop();
        send({type:'error',message:'Unable to load or run the recipe parser. Please try again.'});
      };
      timer = setTimeout(() => {
        if (disposed || generation !== token) return;
        stop();
        send({type:'error',message:'Recipe parsing timed out. Try copying a smaller recipe selection.'});
      }, 20000);
      send({type:'parsing'});
      worker.postMessage(payload);
    } catch (error) {
      stop();
      send({type:'error',message:error.message || 'Unable to start the recipe parser.'});
    }
  };
  textarea.addEventListener('paste', paste);
  textarea.addEventListener('input', input);
  element.addEventListener('noms-source-change', sourceChange);
  element.addEventListener('noms-preview', preview);
  // The UI must not accept a paste until both clipboard representations can be retained.
  send({type:'ready'});
  return () => {
    disposed = true;
    stop();
    textarea.removeEventListener('paste', paste);
    textarea.removeEventListener('input', input);
    element.removeEventListener('noms-source-change', sourceChange);
    element.removeEventListener('noms-preview', preview);
  };
}
