import { lazy, Suspense, useEffect, useState } from 'react'

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

  useEffect(() => {
    const bump = () => setReloadKey((k) => k + 1)
    frame.addEventListener('load', bump)
    return () => frame.removeEventListener('load', bump)
  }, [frame])

  return (
    <div data-testid="comments-mount">
      <Suspense fallback={null}>
        <CommentsApp key={reloadKey} slug={slug} frame={frame} />
      </Suspense>
    </div>
  )
}
