import { useState } from 'react'
import { Eye, EyeOff } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { CopyButton } from './copy-button'

interface PinFieldProps {
  pin: string | null
  editable?: boolean
  onChange?: (value: string) => void
  disabled?: boolean
}

export function PinField({ pin, editable = false, onChange, disabled = false }: Readonly<PinFieldProps>) {
  const { t } = useTranslation()
  const [revealed, setRevealed] = useState(false)

  if (pin === null) return null

  if (editable) {
    return (
      <input
        type="text"
        value={pin}
        disabled={disabled}
        maxLength={6}
        inputMode="numeric"
        onChange={(e) => {
          const filtered = e.target.value.replace(/\D/g, '').slice(0, 6)
          onChange?.(filtered)
        }}
        aria-label={t('form.pin')}
        className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
      />
    )
  }

  return (
    <span className="flex items-center gap-1">
      <span className="font-mono text-sm">{revealed ? pin : '••••••'}</span>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label={revealed ? t('detail.hide_pin') : t('detail.reveal_pin')}
        onClick={() => setRevealed((v) => !v)}
      >
        {revealed ? <EyeOff /> : <Eye />}
      </Button>
      <CopyButton text={pin} ariaLabel={t('detail.copy_pin_aria')} />
    </span>
  )
}
