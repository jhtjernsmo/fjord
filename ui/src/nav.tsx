// App-wide navigation, so deep components (link chips, backlinks) can open things.
import { createContext, useContext } from 'react'
import type { Project } from './api'

export interface Nav {
  openProject: (id: number) => void
  openTask: (projectId: number, taskId: number) => void
  /** Opens a note: in its project's Notes tab, or in the notespace. */
  openNote: (id: number, projectId: number | null) => void
  /** Creates a free note with this title (for links to notes that don't exist yet). */
  createNote: (title: string) => void
  /** Opens the edit dialog for a project (name, description, icon, colour). */
  editProject: (project: Project) => void
}

const noop = () => undefined
export const NavContext = createContext<Nav>({ openProject: noop, openTask: noop, openNote: noop, createNote: noop, editProject: noop })

export function useNav(): Nav {
  return useContext(NavContext)
}
