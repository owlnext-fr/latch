import { useTranslation } from 'react-i18next'
import { locales } from '@/i18n'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import 'flag-icons/css/flag-icons.min.css'

export function LanguageSelect() {
  const { t, i18n } = useTranslation()
  const current = i18n.language.slice(0, 2)

  return (
    <Select value={current} onValueChange={(code) => void i18n.changeLanguage(code)}>
      <SelectTrigger className="w-full" aria-label={t('settings.language')}>
        <SelectValue placeholder={t('settings.language')} />
      </SelectTrigger>
      <SelectContent>
        {locales.map((l) => (
          <SelectItem key={l.code} value={l.code}>
            <span className={`fi fi-${l.flag.toLowerCase()}`} aria-hidden="true" />
            {l.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
