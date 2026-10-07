// Technical, emoji-free iconography: terminal-style glyph badges for projects,
// Lucide line icons for the interface.
import { File, FileArchive, FileAudio, FileCode, FileImage, FileText, FileVideo } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

/** Glyphs offered for projects; any short text works as an icon. */
export const PROJECT_GLYPHS = ['>_', '{}', '</>', 'λ', '#', '$', '~/', '::', '[]', '∆', '◆', '%', '&&', '0x', '#!', '@']

export function ProjectGlyph({ glyph, color, size = 'sm' }: { glyph: string; color: string; size?: 'sm' | 'lg' }) {
  return (
    <span className={`glyph glyph-${size}`} style={{ ['--c' as string]: color }} aria-hidden="true">
      {glyph.slice(0, 3)}
    </span>
  )
}

export function AgentTag({ title }: { title?: string }) {
  return (
    <span className="agent-tag" title={title}>
      AI
    </span>
  )
}

const FILE_TYPES: [RegExp, LucideIcon][] = [
  [/\.(png|jpe?g|gif|webp|svg|heic)$/i, FileImage],
  [/\.(zip|tar|gz|7z|rar|xz)$/i, FileArchive],
  [/\.(mp4|mov|webm|mkv)$/i, FileVideo],
  [/\.(mp3|wav|flac|ogg|m4a)$/i, FileAudio],
  [/\.(js|ts|tsx|jsx|rs|py|swift|kt|java|json|html|css|sh|toml|yml|yaml)$/i, FileCode],
  [/\.(md|txt|rtf|pdf|docx?|odt)$/i, FileText],
]

export function FileTypeIcon({ name, size = 28 }: { name: string; size?: number }) {
  const Icon = FILE_TYPES.find(([re]) => re.test(name))?.[1] ?? File
  return <Icon size={size} strokeWidth={1.4} aria-hidden="true" />
}
