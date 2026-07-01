import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { AnchorStatus } from '../anchor/resolve'
import type { CommentPin } from '../data/adapter'
import { COMMENT_FLUO } from './colors'
import { firstLetter } from './pin-label'
import { timeAgo } from './time-ago'

interface CommentsDrawerProps {
  open: boolean
  pins: CommentPin[]
  statusOf: (pinId: number) => AnchorStatus | undefined
  onClose: () => void
  onSelect: (pinId: number) => void
}

/** Tri : threads sains d'abord (récence desc), orphelins/déplacés en bas. */
export function sortPins(
  pins: CommentPin[],
  statusOf: (id: number) => AnchorStatus | undefined,
): CommentPin[] {
  const isWarning = (p: CommentPin) => {
    const status = statusOf(p.id)
    return status === 'orphaned' || status === 'approximate' ? 1 : 0
  }
  return [...pins].sort((a, b) => {
    const delta = isWarning(a) - isWarning(b)
    return delta !== 0 ? delta : b.created_at.localeCompare(a.created_at)
  })
}

export function CommentsDrawer({
  open,
  pins,
  statusOf,
  onClose,
  onSelect,
}: Readonly<CommentsDrawerProps>) {
  const { t, i18n } = useTranslation()
  // Lazy init : évite d'appeler `Date.now()` (impur) à chaque render (react-hooks/purity).
  const [now] = useState(() => Date.now())
  if (!open) return null
  const ordered = sortPins(pins, statusOf)
  return (
    <aside
      data-testid="comments-drawer"
      className="bg-background fixed inset-y-0 right-0 z-[60] flex w-80 flex-col border-l shadow-xl"
    >
      <header className="flex items-center justify-between border-b px-4 py-3">
        <h2 className="text-sm font-semibold">
          {t('comment.drawer.title', { count: pins.length })}
        </h2>
        <Button
          variant="ghost"
          size="sm"
          aria-label={t('comment.drawer.close')}
          onClick={onClose}
        >
          <X className="size-4" />
        </Button>
      </header>
      {ordered.length === 0 ? (
        <p className="text-muted-foreground p-4 text-sm">
          {t('comment.drawer.empty')}
        </p>
      ) : (
        <ul className="flex-1 overflow-y-auto">
          {ordered.map((pin) => {
            const author = pin.messages[0]?.author_name ?? ''
            const status = statusOf(pin.id)
            const warning = status === 'orphaned' || status === 'approximate'
            const replies = Math.max(0, pin.messages.length - 1)
            return (
              <li key={pin.id}>
                <button
                  type="button"
                  data-testid="drawer-row"
                  onClick={() => onSelect(pin.id)}
                  className="hover:bg-muted flex w-full gap-3 border-b px-4 py-3 text-left"
                >
                  <span
                    className="flex size-7 shrink-0 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-sm"
                    style={{ background: warning ? '#f59e0b' : COMMENT_FLUO }}
                  >
                    {firstLetter(author)}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-2">
                      <span className="truncate text-xs font-semibold">
                        {author}
                      </span>
                      <span className="text-muted-foreground text-[10px]">
                        {timeAgo(
                          pin.messages[0]?.created_at ?? pin.created_at,
                          now,
                          i18n.language,
                        )}
                      </span>
                      {warning && (
                        <span className="rounded-full bg-amber-100 px-1.5 py-0.5 text-[9px] text-amber-700">
                          {status === 'orphaned'
                            ? t('comment.drawer.orphaned')
                            : t('comment.drawer.moved')}
                        </span>
                      )}
                    </span>
                    <span className="text-muted-foreground block truncate text-xs">
                      {pin.messages[0]?.body ?? ''}
                    </span>
                    <span className="text-muted-foreground mt-0.5 block text-[10px]">
                      {t('comment.drawer.replies', { count: replies })}
                    </span>
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </aside>
  )
}
