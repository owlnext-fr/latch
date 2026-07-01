import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getStoredName, setStoredName } from './name-prompt'
import { useFloatingPoint } from './use-floating-point'

const MAX_BODY = 2000

interface ComposePopupProps {
  point: { x: number; y: number }
  submitting: boolean
  onSubmit: (v: { author_name: string; body: string }) => void
  onCancel: () => void
}

export function ComposePopup({ point, submitting, onSubmit, onCancel }: Readonly<ComposePopupProps>) {
  const { t } = useTranslation()
  const { ref, style } = useFloatingPoint(point)
  const [name, setName] = useState(getStoredName())
  const [body, setBody] = useState('')
  const [error, setError] = useState<string | null>(null)

  function submit() {
    const trimmedName = name.trim()
    const trimmedBody = body.trim()
    if (!trimmedName) return setError(t('comment.error.name_required'))
    if (!trimmedBody) return setError(t('comment.error.body_required'))
    if (trimmedBody.length > MAX_BODY) return setError(t('comment.error.body_too_long'))
    setStoredName(trimmedName)
    onSubmit({ author_name: trimmedName, body: trimmedBody })
  }

  return (
    <div
      ref={ref}
      style={style}
      className="bg-background z-[60] w-72 rounded-lg border p-3 shadow-xl"
    >
      <div className="flex flex-col gap-2">
        <Label htmlFor="comment-name">{t('comment.compose.name_label')}</Label>
        <Input
          id="comment-name"
          value={name}
          placeholder={t('comment.compose.name_placeholder')}
          onChange={(e) => { setName(e.target.value); setError(null) }}
        />
        <Label htmlFor="comment-body">{t('comment.compose.body_label')}</Label>
        <Textarea
          id="comment-body"
          value={body}
          placeholder={t('comment.compose.body_placeholder')}
          onChange={(e) => { setBody(e.target.value); setError(null) }}
        />
        {error && <p className="text-destructive text-xs">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onCancel}>
            {t('comment.compose.cancel')}
          </Button>
          <Button type="button" loading={submitting} onClick={submit}>
            {t('comment.compose.submit')}
          </Button>
        </div>
      </div>
    </div>
  )
}
