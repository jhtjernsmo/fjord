// ```mermaid blocks in notes and task descriptions, drawn as diagrams.
// Mermaid is large, so it's only loaded the first time a diagram is shown.
import { useEffect, useRef, useState } from 'react'
import { TriangleAlert } from 'lucide-react'
import { useThemes } from '../themes'

const RENDER_DELAY_MS = 250

let nextId = 0
let mermaidModule: Promise<typeof import('mermaid')['default']> | null = null

function loadMermaid() {
  mermaidModule ??= import('mermaid').then((m) => m.default)
  return mermaidModule
}

export function MermaidDiagram({ code }: { code: string }) {
  const { current } = useThemes()
  const [svg, setSvg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const id = useRef(`mermaid-${nextId++}`)

  useEffect(() => {
    let cancelled = false
    // Wait for typing to pause, so half-written diagrams don't flash errors.
    const timer = window.setTimeout(async () => {
      try {
        const mermaid = await loadMermaid()
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: current.dark ? 'dark' : 'default',
          fontFamily: 'inherit',
          themeVariables: { primaryColor: current.colors.panel2, lineColor: current.colors.muted },
        })
        const { svg } = await mermaid.render(id.current, code)
        if (!cancelled) {
          setSvg(svg)
          setError(null)
        }
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err))
        // Mermaid leaves a broken element behind on errors.
        document.getElementById(`d${id.current}`)?.remove()
      }
    }, RENDER_DELAY_MS)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [code, current])

  if (error) {
    return (
      <div className="mermaid-error">
        <div className="hint warn">
          <TriangleAlert size={13} /> {error.split('\n')[0]}
        </div>
        <pre>
          <code>{code}</code>
        </pre>
      </div>
    )
  }
  if (!svg) return <div className="mermaid-diagram loading" />
  // Rendered with securityLevel 'strict', which sanitises the SVG.
  return <div className="mermaid-diagram" dangerouslySetInnerHTML={{ __html: svg }} />
}
