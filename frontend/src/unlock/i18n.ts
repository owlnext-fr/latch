import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

const en = {
  'unlock.title_brand': 'Prototype prepared for {{brand}}',
  'unlock.title_neutral': 'Protected prototype',
  'unlock.instructions':
    'An access code was shared with this link. Enter it to unlock the prototype.',
  'unlock.pin_label': 'Access code',
  'unlock.submit': 'Unlock',
  'unlock.error_wrong': 'Incorrect code.',
  'unlock.error_throttled': 'Too many attempts. Please try again in a moment.',
  'unlock.error_generic': 'Something went wrong. Please try again.',
}
const fr = {
  'unlock.title_brand': 'Prototype préparé pour {{brand}}',
  'unlock.title_neutral': 'Prototype protégé',
  'unlock.instructions':
    "Un code d'accès vous a été transmis avec ce lien. Saisissez-le pour déverrouiller le prototype.",
  'unlock.pin_label': "Code d'accès",
  'unlock.submit': 'Déverrouiller',
  'unlock.error_wrong': 'Code incorrect.',
  'unlock.error_throttled': 'Trop de tentatives. Réessaie dans un moment.',
  'unlock.error_generic': "Une erreur s'est produite. Réessaie.",
}

const instance = i18next.createInstance()
instance
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: { en: { translation: en }, fr: { translation: fr } },
    fallbackLng: 'en',
    supportedLngs: ['en', 'fr'],
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: { order: ['localStorage', 'navigator'], lookupLocalStorage: 'latch.locale' },
  })

export default instance
