import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'

const LOCALES = ['en', 'fr'] as const
type Locale = (typeof LOCALES)[number]

export function LocaleSwitcher() {
  const { i18n } = useTranslation()
  const current = i18n.language.slice(0, 2) as Locale

  return (
    <fieldset className="flex items-center gap-1 border-0 p-0 m-0">
      <legend className="sr-only">Language</legend>
      {LOCALES.map((locale) => (
        <Button
          key={locale}
          type="button"
          variant={current === locale ? 'secondary' : 'ghost'}
          size="xs"
          aria-pressed={current === locale}
          onClick={() => void i18n.changeLanguage(locale)}
        >
          {locale.toUpperCase()}
        </Button>
      ))}
    </fieldset>
  )
}
