import { useState } from 'react'
import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import type { Capabilities, CommentMessage, CommentPin } from '../data/adapter'
import type { PinPosition } from '../follow/controller'
import { useFloatingRect } from './use-floating-rect'

interface ThreadPopupProps {
  pin: CommentPin
  position: PinPosition
  capabilities: Capabilities
  busy: boolean
  onReply: (body: string) => void
  onEdit: (messageId: number, body: string) => void
  onDelete: (messageId: number) => void
  onDeletePin: () => void
  onClose: () => void
}

export function ThreadPopup(props: Readonly<ThreadPopupProps>) {
  const { pin, position, capabilities, busy, onReply, onEdit, onDelete, onDeletePin, onClose } = props
  const { t } = useTranslation()
  const { ref, style } = useFloatingRect(position.rect)
  const [reply, setReply] = useState('')
  const [editingId, setEditingId] = useState<number | null>(null)
  const [editBody, setEditBody] = useState('')

  function startEdit(m: CommentMessage) {
    setEditingId(m.id)
    setEditBody(m.body)
  }

  function commitEdit() {
    if (editingId !== null && editBody.trim()) onEdit(editingId, editBody.trim())
    setEditingId(null)
  }

  return (
    <div
      ref={ref}
      style={style}
      data-status={position.status}
      className="bg-background z-[60] flex w-80 flex-col gap-3 rounded-lg border p-3 shadow-xl"
    >
      <div className="flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          aria-label={t('comment.thread.close')}
          onClick={onClose}
        >
          <X className="size-4" />
        </Button>
      </div>
      {position.status !== 'anchored' && (
        <p className="text-xs text-amber-600">
          {position.status === 'orphaned'
            ? t('comment.thread.orphaned')
            : t('comment.thread.moved')}
        </p>
      )}
      <ul className="flex max-h-64 flex-col gap-3 overflow-y-auto">
        {pin.messages.map((m) => (
          <li key={m.id} className="flex flex-col gap-1">
            <span className="text-xs font-semibold">{m.author_name}</span>
            {editingId === m.id ? (
              <div className="flex flex-col gap-1">
                <Textarea value={editBody} onChange={(e) => setEditBody(e.target.value)} />
                <div className="flex justify-end gap-2">
                  <Button type="button" variant="ghost" onClick={() => setEditingId(null)}>
                    {t('comment.thread.cancel')}
                  </Button>
                  <Button type="button" loading={busy} onClick={commitEdit}>
                    {t('comment.thread.save')}
                  </Button>
                </div>
              </div>
            ) : (
              <>
                <p className="whitespace-pre-wrap text-sm">{m.body}</p>
                {m.editable && capabilities.canEditOwn && (
                  <div className="flex gap-2">
                    <Button type="button" variant="ghost" size="sm" onClick={() => startEdit(m)}>
                      {t('comment.thread.edit')}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => onDelete(m.id)}
                    >
                      {t('comment.thread.delete')}
                    </Button>
                  </div>
                )}
              </>
            )}
          </li>
        ))}
      </ul>
      {capabilities.canAuthor && (
        <div className="flex flex-col gap-1">
          <Textarea
            value={reply}
            placeholder={t('comment.thread.reply_placeholder')}
            onChange={(e) => setReply(e.target.value)}
          />
          <div className="flex justify-end">
            <Button
              type="button"
              loading={busy}
              onClick={() => {
                if (reply.trim()) {
                  onReply(reply.trim())
                  setReply('')
                }
              }}
            >
              {t('comment.thread.reply_submit')}
            </Button>
          </div>
        </div>
      )}
      {capabilities.canEditOwn && (pin.messages[0]?.editable ?? false) && (
        <div className="flex justify-end">
          <Button type="button" variant="ghost" size="sm" onClick={onDeletePin}>
            {t('comment.thread.delete_thread')}
          </Button>
        </div>
      )}
    </div>
  )
}
