import { useEffect } from 'react'

/**
 * Reflète la langue active sur `<html lang>` (les shells HTML sont figés en
 * `lang="en"` au build ; ce hook resynchronise l'attribut avec l'i18n runtime,
 * pour l'accessibilité et le rendu des lecteurs d'écran).
 */
export function useDocumentLang(lang: string) {
  useEffect(() => {
    document.documentElement.lang = lang
  }, [lang])
}
