import { useTranslation } from 'react-i18next'
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { useDeleteVersion } from '@/hooks/use-projects'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

interface DeleteVersionPanelProps {
  projectId: number
  version: VersionItem
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function DeleteVersionPanel({
  projectId,
  version,
  open,
  onOpenChange,
}: DeleteVersionPanelProps) {
  const { t } = useTranslation()
  const deleteVersion = useDeleteVersion()

  function handleDelete() {
    deleteVersion.mutate(
      { id: projectId, n: version.n },
      {
        onSuccess: () => {
          onOpenChange(false)
        },
      },
    )
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto border-l-destructive/30 sm:max-w-md">
        <SheetHeader>
          <SheetTitle className="text-destructive">
            {t('danger.del_version_title', { n: version.n })}
          </SheetTitle>
        </SheetHeader>

        <div className="flex flex-col gap-4 p-4">
          <p className="text-sm">{t('danger.del_version_intro')}</p>
        </div>

        <SheetFooter className="flex-row justify-end gap-2 p-4">
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
          >
            {t('common.cancel')}
          </Button>
          <Button
            type="button"
            variant="destructive"
            onClick={handleDelete}
            disabled={deleteVersion.isPending}
          >
            {deleteVersion.isPending
              ? t('danger.deleting')
              : t('danger.del_version_confirm')}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}
