import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles.css'
import App from './App'
import { AppProvider } from './data'
import { MenuProvider } from './components/Menus'
import { ThemeProvider } from './themes'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <AppProvider>
        <MenuProvider>
          <App />
        </MenuProvider>
      </AppProvider>
    </ThemeProvider>
  </StrictMode>,
)
