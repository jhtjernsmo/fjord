// Images pasted into Azure Boards work items point at Azure, which needs a sign-in.
// The import downloads them; these helpers find the local copy for an image URL.
import type { ExternalFile } from './api'

const ATTACHMENT_RE = /\/_apis\/wit\/attachments\/([0-9a-f-]{32,})/i

/** The attachment GUID in an Azure image URL, lowercase, or null for other images. */
export function azureAttachmentId(src: string | undefined): string | null {
  const m = src?.match(ATTACHMENT_RE)
  return m ? m[1].toLowerCase() : null
}

/** GUID → attachment id, for files still attached to the task. */
export function localCopies(files: ExternalFile[]): Map<string, number> {
  return new Map(files.flatMap((f) => (f.attachment_id === null ? [] : [[f.external_id.toLowerCase(), f.attachment_id] as const])))
}
