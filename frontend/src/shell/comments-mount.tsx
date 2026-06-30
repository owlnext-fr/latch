import { lazy, Suspense, useEffect, useMemo, useState } from 'react'
import { createVisitorAdapter } from '@/comments/data/visitor-adapter'

const CommentsApp = lazy(() => import('@/comments'))

interface CommentsMountProps {
  slug: string
  frame: HTMLIFrameElement
}

/**
 * Monte la couche commentaire en lazy (chunk Vite séparé du bundle shell).
 * Se ré-instancie au `load` de l'iframe : nouvelle scène/navigation interne du proto
 * → on reconstruit picker + contrôleur sur le DOM courant.
 */
export function CommentsMount({ slug, frame }: Readonly<CommentsMountProps>) {
  const [reloadKey, setReloadKey] = useState(0)
  const adapter = useMemo(() => createVisitorAdapter(slug), [slug])

  useEffect(() => {
    const bump = () => setReloadKey((k) => k + 1)
    frame.addEventListener('load', bump)
    return () => frame.removeEventListener('load', bump)
  }, [frame])

  return (
    <div data-testid="comments-mount">
      <Suspense fallback={null}>
        <CommentsApp key={reloadKey} cacheKey={slug} frame={frame} adapter={adapter} />
      </Suspense>
    </div>
  )
}
