// Kanban board: mouse drag & drop (dnd-kit) and vim-style keyboard control.
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { SortableContext, useSortable, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import confetti from 'canvas-confetti'
import { api, isOverdue, PRIORITIES } from '../api'
import type { Board as BoardData, Status, Task } from '../api'
import { useActions, useApp } from '../data'

const PRIORITY_ICON = ['', '↓', '→', '↑']
const DRAG_ACTIVATION_PX = 5

interface Props {
  board: BoardData
  selectedTaskId: number | null
  onOpenTask: (id: number) => void
  /** Bumped by the parent to focus the quick-add field of the cursor column. */
  quickAddSignal: number
  /** Off while a panel, palette or dialog is open, so h/j/k/x/dd can't fire behind it. */
  keysEnabled: boolean
}

function TaskCard({ task, statuses, cursor, onOpen }: { task: Task; statuses: Status[]; cursor: boolean; onOpen: () => void }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: task.id })
  const ref = useRef<HTMLDivElement | null>(null)
  useEffect(() => {
    if (cursor) ref.current?.scrollIntoView({ block: 'nearest' })
  }, [cursor])
  return (
    <div
      ref={(el) => {
        setNodeRef(el)
        ref.current = el
      }}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      {...attributes}
      {...listeners}
      tabIndex={-1}
      onClick={onOpen}
    >
      <CardBody task={task} statuses={statuses} className={`${cursor ? 'cursor' : ''} ${isDragging ? 'dragging' : ''}`} />
    </div>
  )
}

function CardBody({ task, statuses, className = '' }: { task: Task; statuses: Status[]; className?: string }) {
  const done = statuses.find((s) => s.id === task.status_id)?.is_done
  return (
    <div className={`card ${done ? 'done' : ''} ${className}`}>
      {task.created_by === 'claude' && (
        <span className="agent-badge" title="Laget av Claude">
          🤖
        </span>
      )}
      <div className="card-title">{task.title}</div>
      {(task.priority > 0 || task.due_at || task.body_md) && (
        <div className="card-meta">
          {task.priority > 0 && (
            <span className={`prio-${task.priority}`} title={`Prioritet: ${PRIORITIES[task.priority]}`}>
              {PRIORITY_ICON[task.priority]} {PRIORITIES[task.priority]}
            </span>
          )}
          {task.due_at && <span className={`chip ${isOverdue(task, statuses) ? 'overdue' : ''}`}>📅 {task.due_at.slice(5)}</span>}
          {task.body_md && <span title="Har beskrivelse">≡</span>}
        </div>
      )}
    </div>
  )
}

function Column(props: {
  status: Status
  tasks: Task[]
  statuses: Status[]
  focused: boolean
  over: boolean
  cursorTaskId: number | null
  onOpen: (id: number) => void
  quickAddRef: (el: HTMLInputElement | null) => void
  onAdd: (title: string) => void
}) {
  const { setNodeRef } = useDroppable({ id: `col-${props.status.id}` })
  const [draft, setDraft] = useState('')
  return (
    <section className={`column ${props.focused ? 'focused' : ''} ${props.over ? 'over' : ''}`}>
      <header className="column-head">
        <span className="dot" style={{ background: props.status.color }} />
        {props.status.name}
        <span className="count">{props.tasks.length}</span>
      </header>
      <SortableContext items={props.tasks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
        <div className="column-body" ref={setNodeRef}>
          {props.tasks.map((t) => (
            <TaskCard key={t.id} task={t} statuses={props.statuses} cursor={t.id === props.cursorTaskId} onOpen={() => props.onOpen(t.id)} />
          ))}
        </div>
      </SortableContext>
      <form
        className="quick-add"
        onSubmit={(e) => {
          e.preventDefault()
          if (draft.trim()) props.onAdd(draft.trim())
          setDraft('')
        }}
      >
        <input
          ref={props.quickAddRef}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === 'Escape' && (e.currentTarget.blur(), setDraft(''))}
          placeholder="+ Legg til oppgave"
          aria-label={`Ny oppgave i ${props.status.name}`}
        />
      </form>
    </section>
  )
}

export function Board({ board, selectedTaskId, onOpenTask, quickAddSignal, keysEnabled }: Props) {
  const { run } = useApp()
  const { statuses, tasks } = board
  const columns = useMemo(
    () => statuses.map((s) => ({ status: s, tasks: tasks.filter((t) => t.status_id === s.id) })),
    [statuses, tasks],
  )
  const [cursor, setCursor] = useState({ col: 0, row: 0 })
  const [dragging, setDragging] = useState<Task | null>(null)
  const [overCol, setOverCol] = useState<number | null>(null)
  const quickAdd = useRef(new Map<number, HTMLInputElement>())
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_PX } }))

  const col = Math.min(cursor.col, Math.max(columns.length - 1, 0))
  const colTasks = columns[col]?.tasks ?? []
  const row = Math.min(cursor.row, Math.max(colTasks.length - 1, 0))
  const current: Task | undefined = colTasks[row]

  // Keep the cursor on the open task when it is opened by mouse.
  useEffect(() => {
    if (selectedTaskId == null) return
    columns.forEach((c, ci) => {
      const ri = c.tasks.findIndex((t) => t.id === selectedTaskId)
      if (ri >= 0) setCursor({ col: ci, row: ri })
    })
  }, [selectedTaskId, columns])

  useEffect(() => {
    if (quickAddSignal > 0) quickAdd.current.get(columns[col]?.status.id ?? -1)?.focus()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quickAddSignal])

  const celebrateIfAllDone = async () => {
    const fresh = await api.getBoard(board.project.id)
    const doneIds = new Set(fresh.statuses.filter((s) => s.is_done).map((s) => s.id))
    if (fresh.tasks.length > 0 && fresh.tasks.every((t) => doneIds.has(t.status_id))) {
      confetti({ particleCount: 140, spread: 75, origin: { y: 0.7 } })
    }
  }

  const moveToColumn = async (task: Task, targetCol: number, beforeId: number | null = null) => {
    const target = columns[targetCol]
    if (!target) return
    await run(api.moveTask(task.id, target.status.id, beforeId))
    if (target.status.is_done) celebrateIfAllDone()
  }

  const reorder = async (delta: -1 | 1) => {
    if (!current) return
    const neighbour = colTasks[row + delta]
    if (!neighbour) return
    // Moving down = place before the task after the neighbour (or last).
    const before = delta === -1 ? neighbour.id : (colTasks[row + 2]?.id ?? null)
    await run(api.moveTask(current.id, current.status_id, before))
    setCursor({ col, row: row + delta })
  }

  const boardActions: Record<string, () => void> = {
      'board.left': () => setCursor({ col: Math.max(col - 1, 0), row }),
      'board.right': () => setCursor({ col: Math.min(col + 1, columns.length - 1), row }),
      'board.down': () => setCursor({ col, row: Math.min(row + 1, colTasks.length - 1) }),
      'board.up': () => setCursor({ col, row: Math.max(row - 1, 0) }),
      'board.first': () => setCursor({ col, row: 0 }),
      'board.last': () => setCursor({ col, row: colTasks.length - 1 }),
      'task.open': () => current && onOpenTask(current.id),
      'task.newHere': () => quickAdd.current.get(columns[col]?.status.id ?? -1)?.focus(),
      'task.moveLeft': () => current && col > 0 && moveToColumn(current, col - 1).then(() => setCursor({ col: col - 1, row })),
      'task.moveRight': () =>
        current && col < columns.length - 1 && moveToColumn(current, col + 1).then(() => setCursor({ col: col + 1, row })),
      'task.moveDown': () => reorder(1),
      'task.moveUp': () => reorder(-1),
      'task.toggleDone': () => {
        if (!current) return
        const isDone = columns[col].status.is_done
        const target = isDone ? 0 : columns.findIndex((c) => c.status.is_done)
        if (target >= 0) moveToColumn(current, target)
      },
      'task.archive': () => current && run(api.archiveTask(current.id, true), `Arkiverte «${current.title}»`),
      'task.priority': () => current && run(api.updateTask(current.id, { priority: (current.priority + 1) % 4 })),
  }
  useActions(keysEnabled ? boardActions : {}, 'board')

  const columnOf = (id: string | number): number => {
    if (typeof id === 'string' && id.startsWith('col-')) return columns.findIndex((c) => `col-${c.status.id}` === id)
    return columns.findIndex((c) => c.tasks.some((t) => t.id === id))
  }

  const onDragStart = (e: DragStartEvent) => setDragging(tasks.find((t) => t.id === e.active.id) ?? null)
  const onDragOver = (e: DragOverEvent) => setOverCol(e.over ? columnOf(e.over.id) : null)
  const onDragEnd = async (e: DragEndEvent) => {
    setDragging(null)
    setOverCol(null)
    const task = tasks.find((t) => t.id === e.active.id)
    if (!task || !e.over) return
    const target = columnOf(e.over.id)
    if (target < 0) return
    const overTask = typeof e.over.id === 'number' ? e.over.id : null
    if (overTask === task.id) return
    // Dropping on a card places the task before it; on empty space, last.
    await moveToColumn(task, target, overTask)
  }

  return (
    <DndContext sensors={sensors} collisionDetection={closestCorners} onDragStart={onDragStart} onDragOver={onDragOver} onDragEnd={onDragEnd}>
      <div className="board">
        {columns.map((c, ci) => (
          <Column
            key={c.status.id}
            status={c.status}
            tasks={c.tasks}
            statuses={statuses}
            focused={ci === col}
            over={ci === overCol}
            cursorTaskId={ci === col ? (current?.id ?? null) : null}
            onOpen={onOpenTask}
            quickAddRef={(el) => (el ? quickAdd.current.set(c.status.id, el) : quickAdd.current.delete(c.status.id))}
            onAdd={(title) => run(api.createTask({ project_id: board.project.id, title, status_id: c.status.id }))}
          />
        ))}
      </div>
      <DragOverlay>{dragging && <CardBody task={dragging} statuses={statuses} className="overlay" />}</DragOverlay>
    </DndContext>
  )
}
