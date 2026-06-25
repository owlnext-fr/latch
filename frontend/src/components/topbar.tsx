import { useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { LocaleSwitcher } from '@/components/locale-switcher'
import { useLogout } from '@/hooks/use-auth'

export function Topbar() {
  const router = useRouter()
  const { t } = useTranslation()
  const logout = useLogout()

  function handleLogout() {
    logout.mutate(undefined, {
      onSettled: () => {
        void router.navigate({ to: '/login' })
      },
    })
  }

  return (
    <header className="flex h-14 items-center justify-between border-b px-4">
      <Button
        type="button"
        variant="link"
        className="text-lg font-bold"
        onClick={() => void router.navigate({ to: '/' })}
      >
        latch
      </Button>
      <div className="flex items-center gap-2">
        <LocaleSwitcher />
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
    </header>
  )
}
