import Markdown from 'react-markdown'
import type { Components } from 'react-markdown'

/**
 * Rendu markdown restreint — barrière XSS unique partagée par l'overlay client
 * et l'aperçu admin. Périmètre autorisé : paragraphes, titres, gras/italique,
 * listes, citation. Interdits : liens, images, code, HTML brut.
 *
 * Le style passe par la prop `components` (classes Tailwind / tokens shadcn),
 * pas par du CSS séparé : Tailwind v4 (preflight) réinitialise titres / listes /
 * citation et le projet n'utilise pas @tailwindcss/typography. `allowedElements`
 * + `skipHtml` restent la barrière de sécurité ; `components` ne fait que styler
 * les éléments déjà autorisés. On ne reprend que `children` (seul prop utile pour
 * ces éléments restreints) → pas de `node` propagé au DOM.
 */
const ALLOWED = [
  'p',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'strong',
  'em',
  'ul',
  'ol',
  'li',
  'blockquote',
]

const COMPONENTS: Components = {
  h1: ({ children }) => (
    <h1 className="mt-4 mb-2 text-lg font-semibold first:mt-0">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="mt-4 mb-2 text-base font-semibold first:mt-0">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="mt-3 mb-1.5 text-sm font-semibold first:mt-0">{children}</h3>
  ),
  h4: ({ children }) => (
    <h4 className="mt-3 mb-1.5 text-sm font-semibold first:mt-0">{children}</h4>
  ),
  h5: ({ children }) => (
    <h5 className="mt-3 mb-1.5 text-sm font-semibold first:mt-0">{children}</h5>
  ),
  h6: ({ children }) => (
    <h6 className="mt-3 mb-1.5 text-sm font-semibold first:mt-0">{children}</h6>
  ),
  p: ({ children }) => (
    <p className="my-2 text-sm leading-relaxed first:mt-0 last:mb-0">{children}</p>
  ),
  ul: ({ children }) => (
    <ul className="my-2 list-disc space-y-1 pl-5">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="my-2 list-decimal space-y-1 pl-5">{children}</ol>
  ),
  li: ({ children }) => <li className="text-sm leading-relaxed">{children}</li>,
  blockquote: ({ children }) => (
    <blockquote className="text-muted-foreground my-2 border-l-2 pl-3 italic">
      {children}
    </blockquote>
  ),
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
}

export function MarkdownView({ source }: Readonly<{ source: string }>) {
  return (
    <Markdown
      skipHtml
      allowedElements={ALLOWED}
      unwrapDisallowed
      components={COMPONENTS}
    >
      {source}
    </Markdown>
  )
}
