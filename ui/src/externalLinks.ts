// Links to the web (and mail) always open in the system browser, never inside
// Fjord's window, wherever they appear: notes, task descriptions, settings, …
import { api } from './api'

const EXTERNAL = /^(https?:|mailto:)/i

/** The external URL a click should open, or null to let the app handle it. */
export function externalHref(target: EventTarget | null): string | null {
  const anchor = target instanceof Element ? target.closest('a[href]') : null
  const href = anchor?.getAttribute('href') ?? ''
  return EXTERNAL.test(href) ? href : null
}

/** Installs one document-wide handler; returns a function that removes it. */
export function routeExternalLinks(doc: Document = document): () => void {
  const onClick = (e: MouseEvent) => {
    const href = externalHref(e.target)
    if (!href) return
    e.preventDefault()
    api.openUrl(href).catch(() => undefined)
  }
  // Left and middle clicks; capture so nothing inside can navigate first.
  doc.addEventListener('click', onClick, true)
  doc.addEventListener('auxclick', onClick, true)
  return () => {
    doc.removeEventListener('click', onClick, true)
    doc.removeEventListener('auxclick', onClick, true)
  }
}
