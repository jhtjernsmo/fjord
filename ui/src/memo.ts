// Remembers a network result for a few minutes, so opening the Overview again doesn't call
// GitHub and Azure DevOps each time (GitHub's search allows 30 requests a minute).

const results = new Map<string, { at: number; value: Promise<unknown> }>()

export const NETWORK_TTL_MS = 5 * 60 * 1000

export function memo<T>(key: string, load: () => Promise<T>, ttl = NETWORK_TTL_MS, now = Date.now()): Promise<T> {
  const hit = results.get(key)
  if (hit && now - hit.at < ttl) return hit.value as Promise<T>
  const value = load()
  results.set(key, { at: now, value })
  // A failed call isn't remembered, so the next visit tries again.
  value.catch(() => results.delete(key))
  return value
}
