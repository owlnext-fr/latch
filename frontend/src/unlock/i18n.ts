import { createBundleI18n } from '@/i18n/create-bundle-i18n'

const instance = createBundleI18n(
  import.meta.glob('../i18n/locales/unlock/*.json', { eager: true }),
)

export default instance
