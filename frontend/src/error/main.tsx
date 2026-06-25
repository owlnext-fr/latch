import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { ErrorPage } from './error-page'
import '@/index.css'

createRoot(document.getElementById('error-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <ErrorPage />
    </I18nextProvider>
  </StrictMode>,
)
