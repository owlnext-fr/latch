/** 1ʳᵉ lettre (majuscule) d'un nom d'auteur ; `•` si vide. */
export function firstLetter(name: string): string {
  const trimmed = name.trim()
  return trimmed ? trimmed[0].toUpperCase() : '•'
}
