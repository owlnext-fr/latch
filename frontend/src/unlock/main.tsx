import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { UnlockPage } from './unlock-page'
import '@/index.css'

createRoot(document.getElementById('unlock-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <UnlockPage />
    </I18nextProvider>
  </StrictMode>,
)
