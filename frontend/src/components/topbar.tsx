import { useState } from 'react'
import { useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { Settings, CircleHelp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Logo } from '@/components/logo'
import { DOCS_URL } from '@/lib/links'
import { SettingsSheet } from '@/components/settings-sheet'
import { useLogout } from '@/hooks/use-auth'

export function Topbar() {
  const router = useRouter()
  const { t } = useTranslation()
  const logout = useLogout()
  const [settingsOpen, setSettingsOpen] = useState(false)

  function handleLogout() {
    logout.mutate(undefined, {
      onSettled: () => {
        router.navigate({ to: '/login' })
      },
    })
  }

  return (
    <header className="flex h-14 items-center justify-between border-b px-4">
      <Button
        type="button"
        variant="link"
        className="gap-2 text-lg font-bold"
        onClick={() => {
          router.navigate({ to: '/' })
        }}
      >
        <Logo className="size-6" />
        latch
      </Button>
      <div className="flex items-center gap-2">
        <Button asChild variant="ghost" size="icon-sm">
          <a
            href={DOCS_URL}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={t('topbar.help')}
          >
            <CircleHelp />
          </a>
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={t('settings.title')}
          onClick={() => setSettingsOpen(true)}
        >
          <Settings />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleLogout}
          loading={logout.isPending}
        >
          {t('common.logout')}
        </Button>
      </div>
      <SettingsSheet open={settingsOpen} onOpenChange={setSettingsOpen} />
    </header>
  )
}
