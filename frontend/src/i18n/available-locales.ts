export type LocaleMeta = { name: string; flag: string }
export type LocaleInfo = { code: string } & LocaleMeta
export type ParsedLocales = {
  resources: Record<string, { translation: Record<string, string> }>
  locales: LocaleInfo[]
}

type GlobModule = { default: Record<string, unknown> }

function codeFromPath(filePath: string): string {
  // glob keys always contain a path separator, so pop() is always a string
  return filePath.split('/').pop()!.replace(/\.json$/, '')
}

function normalizeMeta(meta: unknown, code: string): LocaleMeta {
  const fallback: LocaleMeta = { name: code.toUpperCase(), flag: code.toUpperCase() }
  if (!meta || typeof meta !== 'object') {
    console.warn(`[i18n] locale "${code}" has no _meta; falling back to "${fallback.name}"`)
    return fallback
  }
  const m = meta as Record<string, unknown>
  return {
    name: typeof m.name === 'string' && m.name ? m.name : fallback.name,
    flag: typeof m.flag === 'string' && m.flag ? m.flag : fallback.flag,
  }
}

/**
 * Transforme le résultat d'un `import.meta.glob('...', { eager: true })` de fichiers
 * locale JSON en ressources i18next + métadonnées de langue. Fonction pure (la
 * découverte glob, primitive Vite, reste chez l'appelant) → unitairement testable.
 */
export function parseLocales(glob: Record<string, GlobModule>): ParsedLocales {
  const resources: ParsedLocales['resources'] = {}
  const locales: LocaleInfo[] = []

  for (const [filePath, mod] of Object.entries(glob)) {
    const code = codeFromPath(filePath)
    const { _meta, ...translation } = mod.default
    // JSON locale files contain only string values by convention (i18next resources)
    resources[code] = { translation: translation as Record<string, string> }
    locales.push({ code, ...normalizeMeta(_meta, code) })
  }

  locales.sort((a, b) => a.code.localeCompare(b.code))
  return { resources, locales }
}
