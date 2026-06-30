import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Trash2 } from 'lucide-react'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import {
  useVersionComments,
  useModerateComment,
} from '@/hooks/use-version-comments'
import type { components } from '@/api/schema'

type AdminCommentPin = components['schemas']['AdminCommentPin']

function anchorLabel(anchorJson: string): string {
  try {
    const a = JSON.parse(anchorJson) as {
      fingerprint?: { tag?: string; text?: string }
    }
    const tag = a.fingerprint?.tag ?? 'element'
    const text = a.fingerprint?.text
    return text ? `${tag} — "${text}"` : tag
  } catch {
    return 'element'
  }
}

interface VersionCommentsPanelProps {
  projectId: number
  version: number
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function VersionCommentsPanel(
  props: Readonly<VersionCommentsPanelProps>,
): JSX.Element {
  const { projectId, version, open, onOpenChange } = props
  const { t } = useTranslation()
  const { data, isLoading } = useVersionComments(projectId, version)
  const moderateComment = useModerateComment(projectId, version)
  const [confirmingId, setConfirmingId] = useState<number | null>(null)

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>
            {t('version_comments.title', { n: version })}
          </SheetTitle>
        </SheetHeader>

        <div className="flex flex-col gap-4 p-4">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">
              {t('version_comments.loading')}
            </p>
          ) : !data || data.pins.length === 0 ? (
            <p className="text-muted-foreground text-sm">
              {t('version_comments.empty')}
            </p>
          ) : (
            data.pins.map((pin: AdminCommentPin) => (
              <div
                key={pin.id}
                className="rounded-md border border-input p-3"
              >
                <p className="text-muted-foreground mb-2 text-xs font-medium">
                  {t('version_comments.anchor_label')}:{' '}
                  {anchorLabel(pin.anchor)}
                </p>
                <ul className="flex flex-col gap-2">
                  {pin.messages.map((m) => (
                    <li key={m.id} className="flex flex-col gap-1">
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex min-w-0 flex-col gap-0.5">
                          <span className="text-xs font-semibold">
                            {m.author_name}
                          </span>
                          <p className="text-sm">{m.body}</p>
                          <span className="text-muted-foreground text-xs">
                            {new Date(m.created_at).toLocaleDateString()}
                          </span>
                        </div>
                        {confirmingId === m.id ? (
                          <div className="flex shrink-0 flex-wrap items-center gap-1">
                            <span className="text-xs">
                              {t('version_comments.confirm_delete')}
                            </span>
                            <Button
                              type="button"
                              variant="destructive"
                              size="sm"
                              onClick={() => {
                                moderateComment.mutate(m.id)
                                setConfirmingId(null)
                              }}
                            >
                              {t('version_comments.confirm_yes')}
                            </Button>
                            <Button
                              type="button"
                              variant="ghost"
                              size="sm"
                              onClick={() => setConfirmingId(null)}
                            >
                              {t('version_comments.confirm_no')}
                            </Button>
                          </div>
                        ) : (
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            aria-label={t('version_comments.delete_aria')}
                            onClick={() => setConfirmingId(m.id)}
                          >
                            <Trash2 />
                          </Button>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
              </div>
            ))
          )}
        </div>
      </SheetContent>
    </Sheet>
  )
}
