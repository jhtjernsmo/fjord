// Subtasks in the task panel: progress, check off, add (Enter), drag to reorder,
// click to open. One level deep: a subtask shows its parent instead.
import { useRef, useState } from 'react'
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { SortableContext, useSortable, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { CornerLeftUp, GripVertical, Plus } from 'lucide-react'
import { api } from '../api'
import type { Status, Task } from '../api'
import { useActions, useApp, useLive } from '../data'
import { useNav } from '../nav'

const DRAG_ACTIVATION_PX = 4

function SubtaskRow({ sub, done, onToggle, onOpen }: { sub: Task; done: boolean; onToggle: () => void; onOpen: () => void }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: sub.id })
  return (
    <div ref={setNodeRef} className={`subtask ${done ? 'done' : ''} ${isDragging ? 'dragging' : ''}`} style={{ transform: CSS.Transform.toString(transform), transition }}>
      <span className="subtask-grip" {...attributes} {...listeners} aria-label="drag">
        <GripVertical size={13} />
      </span>
      <input type="checkbox" checked={done} onChange={onToggle} aria-label={sub.title} />
      <button className="subtask-title" onClick={onOpen}>
        {sub.title}
      </button>
    </div>
  )
}

export function Subtasks({ task, statuses }: { task: Task; statuses: Status[] }) {
  const { run, t } = useApp()
  const nav = useNav()
  const [subtasks] = useLive(() => api.listSubtasks(task.id), [task.id])
  const [parent] = useLive(async () => (task.parent_id ? ((await api.getBoard(task.project_id)).tasks.find((x) => x.id === task.parent_id) ?? null) : null), [task.parent_id])
  const [draft, setDraft] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_PX } }))

  const doneIds = new Set(statuses.filter((s) => s.is_done).map((s) => s.id))
  const doneColumn = statuses.find((s) => s.is_done)
  const firstColumn = statuses[0]

  useActions(task.parent_id ? {} : { 'task.addSubtask': () => inputRef.current?.focus() })

  // A subtask shows where it belongs instead of its own list.
  if (task.parent_id) {
    return parent ? (
      <button className="parent-link" onClick={() => nav.openTask(task.project_id, parent.id)}>
        <CornerLeftUp size={13} /> {t('subtasks.of', { title: parent.title })}
      </button>
    ) : null
  }

  const items = subtasks ?? []
  const doneCount = items.filter((s) => doneIds.has(s.status_id)).length
  const allDone = items.length > 0 && doneCount === items.length
  const parentDone = doneIds.has(task.status_id)

  const toggle = (sub: Task) => {
    const target = doneIds.has(sub.status_id) ? firstColumn : doneColumn
    if (target) run(api.moveTask(sub.id, target.id))
  }
  const add = (e: React.FormEvent) => {
    e.preventDefault()
    const title = draft.trim()
    if (!title) return
    setDraft('')
    run(api.createTask({ project_id: task.project_id, title, parent_id: task.id }))
  }
  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return
    const index = items.findIndex((s) => s.id === e.over?.id)
    if (index >= 0) run(api.moveSubtask(Number(e.active.id), index))
  }

  return (
    <div className="subtasks">
      <div className="section-title">
        {t('subtasks.title')}
        {items.length > 0 && (
          <span className="subtask-progress">
            <span className="bar">
              <span style={{ width: `${(doneCount / items.length) * 100}%` }} />
            </span>
            {doneCount}/{items.length}
          </span>
        )}
      </div>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={items.map((s) => s.id)} strategy={verticalListSortingStrategy}>
          {items.map((sub) => (
            <SubtaskRow key={sub.id} sub={sub} done={doneIds.has(sub.status_id)} onToggle={() => toggle(sub)} onOpen={() => nav.openTask(task.project_id, sub.id)} />
          ))}
        </SortableContext>
      </DndContext>
      <form className="subtask-add" onSubmit={add}>
        <Plus size={13} />
        <input ref={inputRef} value={draft} onChange={(e) => setDraft(e.target.value)} placeholder={t('subtasks.add')} aria-label={t('subtasks.add')} onKeyDown={(e) => e.key === 'Escape' && e.currentTarget.blur()} />
        <kbd>A</kbd>
      </form>
      {allDone && !parentDone && doneColumn && (
        <div className="subtask-hint">
          {t('subtasks.allDone')}
          <button className="btn ghost" onClick={() => run(api.moveTask(task.id, doneColumn.id))}>
            {t('subtasks.moveParent', { column: doneColumn.name })}
          </button>
        </div>
      )}
    </div>
  )
}
