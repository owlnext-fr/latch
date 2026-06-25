import { Copy } from 'lucide-react'
import { toast } from 'sonner'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'

interface CopyButtonProps {
  text: string
  ariaLabel: string
}

export function CopyButton({ text, ariaLabel }: CopyButtonProps) {
  const { t } = useTranslation()

  async function handleClick() {
    await navigator.clipboard.writeText(text)
    toast.success(t('toast.copied'))
  }

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label={ariaLabel}
      onClick={() => void handleClick()}
    >
      <Copy />
    </Button>
  )
}
