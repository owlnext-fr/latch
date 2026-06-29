import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { ShellPage } from './shell-page'
import '@/index.css'

createRoot(document.getElementById('shell-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <ShellPage />
    </I18nextProvider>
  </StrictMode>,
)
