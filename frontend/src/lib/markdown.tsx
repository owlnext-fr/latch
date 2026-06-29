import Markdown from 'react-markdown'

/**
 * Rendu markdown restreint — barrière XSS unique partagée par l'overlay client
 * et l'aperçu admin. Périmètre autorisé : paragraphes, titres, gras/italique,
 * listes, citation. Interdits : liens, images, code, HTML brut.
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

export function MarkdownView({ source }: Readonly<{ source: string }>) {
  return (
    <Markdown
      skipHtml
      allowedElements={ALLOWED}
      unwrapDisallowed
    >
      {source}
    </Markdown>
  )
}
