// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.hoisted(() => vi.fn(async () => null))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

import { routeExternalLinks } from '../src/externalLinks'

afterEach(() => {
  document.body.innerHTML = ''
  invoke.mockClear()
})

function click(el: Element): MouseEvent {
  const e = new MouseEvent('click', { bubbles: true, cancelable: true })
  el.dispatchEvent(e)
  return e
}

describe('external links', () => {
  it('open web and mail links in the browser instead of the app window', () => {
    const stop = routeExternalLinks()
    document.body.innerHTML = '<p><a href="https://example.com/x"><b>docs</b></a> <a href="mailto:a@b.no">mail</a> <a href="http://intranet/">old</a></p>'
    // Clicking inside the link (the <b>) counts too.
    expect(click(document.querySelector('b') as Element).defaultPrevented).toBe(true)
    for (const a of [...document.querySelectorAll('a')].slice(1)) expect(click(a).defaultPrevented).toBe(true)
    expect(invoke.mock.calls.map((c) => (c as unknown[])[1])).toEqual([
      { url: 'https://example.com/x' },
      { url: 'mailto:a@b.no' },
      { url: 'http://intranet/' },
    ])
    stop()
  })

  it('leaves in-app links and plain clicks alone', () => {
    const stop = routeExternalLinks()
    document.body.innerHTML = '<a href="fjord-link:Bokost">chip</a><a href="#top">top</a><button>x</button>'
    for (const el of document.querySelectorAll('a, button')) expect(click(el).defaultPrevented).toBe(false)
    expect(invoke).not.toHaveBeenCalled()
    stop()
  })
})
