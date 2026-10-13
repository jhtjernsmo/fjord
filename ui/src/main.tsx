import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles.css'
import App from './App'
import { AppProvider } from './data'
import { MenuProvider } from './components/Menus'
import { ThemeProvider } from './themes'
import { routeExternalLinks } from './externalLinks'
import { TaskWindow } from './components/TaskWindow'

routeExternalLinks()

// A popped-out task window loads index.html?task=<id>.
const poppedOutTask = Number(new URLSearchParams(location.search).get('task')) || null

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <AppProvider>
        <MenuProvider>
          {poppedOutTask ? <TaskWindow taskId={poppedOutTask} /> : <App />}
        </MenuProvider>
      </AppProvider>
    </ThemeProvider>
  </StrictMode>,
)
