import { useTranslation } from 'react-i18next'
import { CircleCheck } from 'lucide-react'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Badge } from '@/components/ui/badge'
import { MarkdownView } from '@/lib/markdown'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

interface VersionDetailPanelProps {
  version: VersionItem
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Détail read-only d'une version : métadonnées + notes de version rendues
 * (MarkdownView, identique à l'overlay visiteur) ou état vide. Fermeture via le
 * bouton X intégré du Sheet.
 */
export function VersionDetailPanel({
  version,
  open,
  onOpenChange,
}: Readonly<VersionDetailPanelProps>) {
  const { t } = useTranslation()

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            {t('version_detail.title', { n: version.n })}
            {version.is_active && (
              <Badge className="bg-green-600 text-white hover:bg-green-600">
                <CircleCheck />
                {t('common.active')}
              </Badge>
            )}
          </SheetTitle>
        </SheetHeader>

        <div className="flex flex-col gap-4 p-4">
          <div>
            <p className="text-muted-foreground mb-0.5 text-xs font-medium">
              {t('version_detail.date_label')}
            </p>
            <p className="text-sm">
              {new Date(version.created_at).toLocaleDateString()}
            </p>
          </div>

          <div>
            <p className="text-muted-foreground mb-1 text-xs font-medium">
              {t('version_detail.notes_label')}
            </p>
            {version.release_notes ? (
              <div className="rounded-md border border-input px-3 py-2">
                <MarkdownView source={version.release_notes} />
              </div>
            ) : (
              <p className="text-muted-foreground text-sm">
                {t('version_detail.no_notes')}
              </p>
            )}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
