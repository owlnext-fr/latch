import { useTranslation } from 'react-i18next'
import { MessageSquarePlus, Eye, List } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { Capabilities } from '../data/adapter'

interface ActionBarProps {
  capabilities: Capabilities
  pinCount: number
  pickActive: boolean
  pinsVisible: boolean
  onTogglePick: () => void
  onToggleVisible: () => void
  onOpenList: () => void
}

export function ActionBar({
  capabilities,
  pinCount,
  pickActive,
  pinsVisible,
  onTogglePick,
  onToggleVisible,
  onOpenList,
}: Readonly<ActionBarProps>) {
  const { t } = useTranslation()
  return (
    <div className="bg-background fixed bottom-4 left-1/2 z-[55] flex -translate-x-1/2 items-center gap-1 rounded-full border p-1 shadow-lg">
      {capabilities.canAuthor && (
        <Button
          type="button"
          variant={pickActive ? 'default' : 'ghost'}
          size="sm"
          onClick={onTogglePick}
        >
          <MessageSquarePlus className="size-4" />
          {t('comment.bar.pick')}
        </Button>
      )}
      <Button
        type="button"
        variant={pinsVisible ? 'default' : 'ghost'}
        size="sm"
        onClick={onToggleVisible}
      >
        <Eye className="size-4" />
        {t('comment.bar.count', { count: pinCount })}
      </Button>
      <Button type="button" variant="ghost" size="sm" onClick={onOpenList}>
        <List className="size-4" />
        {t('comment.bar.list')}
      </Button>
    </div>
  )
}
