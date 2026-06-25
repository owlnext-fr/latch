import { useTranslation } from 'react-i18next'
import { useRouter } from '@tanstack/react-router'
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { useDeleteProject } from '@/hooks/use-projects'
import type { components } from '@/api/schema'

type ProjectDetail = components['schemas']['ProjectDetail']

interface DeleteProjectPanelProps {
  project: ProjectDetail
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function DeleteProjectPanel({
  project,
  open,
  onOpenChange,
}: DeleteProjectPanelProps) {
  const { t } = useTranslation()
  const router = useRouter()
  const deleteProject = useDeleteProject()

  function handleDelete() {
    deleteProject.mutate(project.id, {
      onSuccess: () => {
        onOpenChange(false)
        void router.navigate({ to: '/' })
      },
    })
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto border-l-destructive/30 sm:max-w-md">
        <SheetHeader>
          <SheetTitle className="text-destructive">
            {t('danger.del_project_title', { name: project.name })}
          </SheetTitle>
        </SheetHeader>

        <div className="flex flex-col gap-4 p-4">
          <p className="text-sm">{t('danger.del_project_intro')}</p>
          <ul className="list-disc space-y-1 pl-5 text-sm">
            <li>{t('danger.del_project_li1')}</li>
            <li>
              {t('danger.del_project_li2', {
                count: project.versions.length,
              })}
            </li>
            <li>{t('danger.del_project_li3')}</li>
          </ul>
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
            loading={deleteProject.isPending}
          >
            {t('danger.del_project_confirm')}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}
