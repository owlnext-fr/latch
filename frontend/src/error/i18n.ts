import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from '@/i18n/available-locales'

const { resources, locales } = parseLocales(
  import.meta.glob('../i18n/locales/error/*.json', { eager: true }),
)

const instance = i18next.createInstance()
instance
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: locales.map((l) => l.code),
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: { order: ['localStorage', 'navigator'], lookupLocalStorage: 'latch.locale' },
  })

export default instance
