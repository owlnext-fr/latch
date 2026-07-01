import { lazy, Suspense, useEffect, useMemo, useState } from 'react'
import { useParams, useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { createAdminAdapter } from '@/comments/data/admin-adapter'
import { previewUrl } from '@/lib/utils'

const CommentsApp = lazy(() => import('@/comments'))

export function ReviewPage() {
  const { t } = useTranslation()
  const router = useRouter()
  const { id: idStr, n: nStr } = useParams({ strict: false }) as {
    id?: string
    n?: string
  }
  const id = idStr ?? '0'
  const n = nStr ?? '0'

  const [frameEl, setFrameEl] = useState<HTMLIFrameElement | null>(null)
  const [reloadKey, setReloadKey] = useState(0)

  const cacheKey = useMemo(() => `admin:${id}:${n}`, [id, n])
  const adapter = useMemo(
    () => createAdminAdapter(Number(id), Number(n), t('comment.admin_author')),
    [id, n, t],
  )

  useEffect(() => {
    if (!frameEl) return
    const bump = () => setReloadKey((k) => k + 1)
    frameEl.addEventListener('load', bump)
    return () => frameEl.removeEventListener('load', bump)
  }, [frameEl])

  return (
    <div className="flex h-svh flex-col">
      <div className="flex shrink-0 items-center border-b px-4 py-2">
        <button
          type="button"
          className="text-sm text-muted-foreground hover:text-foreground"
          onClick={() =>
            router.navigate({ to: '/projects/$id', params: { id } })
          }
        >
          {t('review.back')}
        </button>
      </div>
      <div className="relative flex-1 overflow-hidden">
        <iframe
          title="Prototype preview"
          src={previewUrl(Number(id), Number(n))}
          ref={setFrameEl}
          className="h-full w-full border-0"
        />
        {frameEl && (
          <Suspense fallback={null}>
            <CommentsApp
              key={reloadKey}
              cacheKey={cacheKey}
              frame={frameEl}
              adapter={adapter}
            />
          </Suspense>
        )}
      </div>
    </div>
  )
}
