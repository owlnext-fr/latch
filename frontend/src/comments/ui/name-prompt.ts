const KEY = 'latch:comment-name'

export function getStoredName(): string {
  try {
    return localStorage.getItem(KEY) ?? ''
  } catch {
    return ''
  }
}

export function setStoredName(name: string): void {
  try {
    localStorage.setItem(KEY, name.trim())
  } catch {
    /* storage indisponible (mode privé) : on ignore, le nom reste en mémoire */
  }
}
