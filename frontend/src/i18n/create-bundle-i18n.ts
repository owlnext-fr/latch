import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from './available-locales'

type GlobModule = { default: Record<string, unknown> }

/**
 * Factory pour les bundles i18n isolés (shell, unlock, error).
 * Chaque bundle crée sa propre instance i18next (pas de partage de state).
 * Le glob est résolu par l'appelant (Vite exige un littéral statique par module).
 */
export function createBundleI18n(glob: Record<string, GlobModule>) {
  const { resources, locales } = parseLocales(glob)
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
  return instance
}
