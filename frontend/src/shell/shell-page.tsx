import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { MarkdownView } from '@/lib/markdown'
import { CommentsMount } from './comments-mount'

/** Slug courant extrait de `/c/<slug>` (1er segment après `/c/`). */
function currentSlug(): string {
  return globalThis.location.pathname.split('/')[2] ?? ''
}

function seenKey(slug: string): string {
  return `latch:seen:${slug}`
}

interface Notes {
  n: number
  notes_md: string
}

export function ShellPage() {
  const { t } = useTranslation()
  const slug = currentSlug()
  const [notes, setNotes] = useState<Notes | null>(null)
  const [iframeEl, setIframeEl] = useState<HTMLIFrameElement | null>(null)
  const [commentsEnabled, setCommentsEnabled] = useState(false)

  useEffect(() => {
    let cancelled = false
    fetch(`/api/public/${slug}`)
      .then((res) => (res.ok ? res.json() : null))
      .then((meta: { comments_enabled?: boolean } | null) => {
        if (!cancelled && meta?.comments_enabled) setCommentsEnabled(true)
      })
      .catch(() => {
        /* best-effort : un échec meta ne doit jamais masquer le proto */
      })
    return () => {
      cancelled = true
    }
  }, [slug])

  useEffect(() => {
    let cancelled = false
    fetch(`/c/${slug}/notes`)
      .then(async (res) => {
        if (res.status !== 200) return null
        return (await res.json()) as Notes
      })
      .then((data) => {
        if (cancelled || !data) return
        const seen = Number(localStorage.getItem(seenKey(slug)) ?? '0')
        if (data.n > seen) setNotes(data)
      })
      .catch(() => {
        /* notes best-effort : un échec ne doit jamais masquer le proto */
      })
    return () => {
      cancelled = true
    }
  }, [slug])

  function dismiss() {
    if (notes) localStorage.setItem(seenKey(slug), String(notes.n))
    setNotes(null)
  }

  return (
    <div className="relative h-svh w-svw">
      <iframe
        title="prototype"
        src={`/c/${slug}/raw`}
        ref={setIframeEl}
        className="h-full w-full border-0"
      />
      {notes && (
        <div className="bg-background/60 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="bg-background w-full max-w-lg rounded-xl border p-6 shadow-xl">
            <h2 className="mb-3 text-lg font-semibold">{t('shell.notes_title')}</h2>
            <div className="max-h-[60vh] overflow-y-auto">
              <MarkdownView source={notes.notes_md} />
            </div>
            <div className="mt-5 flex justify-end">
              <Button type="button" onClick={dismiss} data-testid="notes-dismiss">
                {t('shell.dismiss')}
              </Button>
            </div>
          </div>
        </div>
      )}
      {commentsEnabled && iframeEl && <CommentsMount slug={slug} frame={iframeEl} />}
    </div>
  )
}
