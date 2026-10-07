import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles.css'
import App from './App'
import { AppProvider } from './data'
import { MenuProvider } from './components/Menus'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AppProvider>
      <MenuProvider>
        <App />
      </MenuProvider>
    </AppProvider>
  </StrictMode>,
)
