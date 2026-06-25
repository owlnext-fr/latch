import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { ThemeProvider } from 'next-themes'
import { Toaster } from '@/components/ui/sonner'
import i18n from '@/i18n'
import { router } from '@/router'
import { setUnauthorizedHandler } from '@/api/client'
import './index.css'

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })

setUnauthorizedHandler(() => {
  queryClient.clear()
  router.navigate({ to: '/login' })
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      storageKey="latch.theme"
      disableTransitionOnChange
    >
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
          <Toaster richColors position="top-right" />
        </QueryClientProvider>
      </I18nextProvider>
    </ThemeProvider>
  </StrictMode>,
)
