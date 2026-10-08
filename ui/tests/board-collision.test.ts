import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { closestCorners } from '@dnd-kit/core'
import type { ClientRect, CollisionDetection } from '@dnd-kit/core'
import { boardCollision } from '../src/components/Board'

const rect = (left: number, top: number, width: number, height: number): ClientRect => ({
  left,
  top,
  width,
  height,
  right: left + width,
  bottom: top + height,
})

// Two tall columns on a big screen; a card sits at the top of column 1.
const droppables = new Map<string, ClientRect>([
  ['col-1', rect(0, 100, 300, 1800)],
  ['col-2', rect(320, 100, 300, 1800)],
  ['5', rect(10, 140, 280, 70)],
])

function args(pointer: { x: number; y: number }, dragged: ClientRect): Parameters<CollisionDetection>[0] {
  const containers = [...droppables.keys()].map((id) => ({ id, rect: { current: droppables.get(id) } }))
  return {
    active: { id: '5' },
    collisionRect: dragged,
    droppableRects: droppables,
    droppableContainers: containers,
    pointerCoordinates: pointer,
  } as unknown as Parameters<CollisionDetection>[0]
}

describe('boardCollision', () => {
  it('picks the column under the pointer, even near the top of a tall column', () => {
    // The card is dragged just over the top of column 2.
    const a = args({ x: 400, y: 170 }, rect(330, 140, 280, 70))
    expect(closestCorners(a)[0]?.id).not.toBe('col-2') // the old behaviour this fixes
    expect(boardCollision(a)[0]?.id).toBe('col-2')
  })

  it('prefers a card under the pointer over its column, and falls back outside columns', () => {
    expect(boardCollision(args({ x: 100, y: 170 }, rect(10, 140, 280, 70)))[0]?.id).toBe('5')
    expect(boardCollision(args({ x: 2000, y: 170 }, rect(1990, 140, 280, 70))).length).toBeGreaterThan(0)
  })
})
