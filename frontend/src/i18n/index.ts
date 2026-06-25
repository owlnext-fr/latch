import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from './available-locales'

const { resources, locales } = parseLocales(
  import.meta.glob('./locales/admin/*.json', { eager: true }),
)

export { locales }

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: locales.map((l) => l.code),
    keySeparator: false, // clés plates "login.title"
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      lookupLocalStorage: 'latch.locale',
      caches: ['localStorage'],
    },
  })

export default i18n
