import { useTranslation } from 'react-i18next'
import { locales } from '@/i18n'
import { Button } from '@/components/ui/button'

export function LocaleSwitcher() {
  const { i18n } = useTranslation()
  const current = i18n.language.slice(0, 2)

  return (
    <fieldset className="m-0 flex items-center gap-1 border-0 p-0">
      <legend className="sr-only">Language</legend>
      {locales.map((l) => (
        <Button
          key={l.code}
          type="button"
          variant={current === l.code ? 'secondary' : 'ghost'}
          size="xs"
          aria-pressed={current === l.code}
          onClick={() => void i18n.changeLanguage(l.code)}
        >
          {l.code.toUpperCase()}
        </Button>
      ))}
    </fieldset>
  )
}
