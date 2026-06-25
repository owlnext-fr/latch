import { useTheme } from 'next-themes'
import { useTranslation } from 'react-i18next'
import { Monitor, Sun, Moon } from 'lucide-react'
import { Button } from '@/components/ui/button'

const OPTIONS = [
  { value: 'system', icon: Monitor, labelKey: 'settings.theme_system' },
  { value: 'light', icon: Sun, labelKey: 'settings.theme_light' },
  { value: 'dark', icon: Moon, labelKey: 'settings.theme_dark' },
] as const

export function ThemeToggle() {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()

  return (
    <fieldset className="m-0 flex items-center gap-1 border-0 p-0">
      <legend className="sr-only">{t('settings.theme')}</legend>
      {OPTIONS.map(({ value, icon: Icon, labelKey }) => (
        <Button
          key={value}
          type="button"
          variant={theme === value ? 'secondary' : 'ghost'}
          size="sm"
          aria-pressed={theme === value}
          onClick={() => setTheme(value)}
        >
          <Icon className="mr-1 size-4" />
          {t(labelKey)}
        </Button>
      ))}
    </fieldset>
  )
}
