import { useState } from 'react'
import { useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { Topbar } from '@/components/topbar'
import { CopyButton } from '@/components/copy-button'
import { ProjectForm } from '@/components/project-form'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { useProjects } from '@/hooks/use-projects'
import { publicUrl } from '@/lib/utils'

export function ListPage() {
  const { t } = useTranslation()
  const router = useRouter()
  const { data: projects, isLoading } = useProjects()
  const [formOpen, setFormOpen] = useState(false)

  return (
    <div className="flex min-h-screen flex-col">
      <Topbar />

      <main className="flex-1 p-6">
        <div className="mb-4 flex items-center justify-between">
          <p className="text-muted-foreground text-sm">{t('list.intro')}</p>
          <Button type="button" onClick={() => setFormOpen(true)}>
            {t('common.new_project')}
          </Button>
        </div>

        {isLoading ? (
          <p className="text-muted-foreground text-sm">{t('common.loading')}</p>
        ) : !projects || projects.length === 0 ? (
          <div className="flex flex-col items-center gap-4 py-16">
            <p className="text-muted-foreground">{t('list.empty')}</p>
            <Button type="button" onClick={() => setFormOpen(true)}>
              {t('list.create_first')}
            </Button>
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('list.col_name')}</TableHead>
                <TableHead>{t('list.col_url')}</TableHead>
                <TableHead>{t('list.col_code')}</TableHead>
                <TableHead>{t('list.col_version')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {projects.map((project) => (
                <TableRow key={project.id}>
                  <TableCell>
                    <button
                      type="button"
                      className="font-medium hover:underline"
                      onClick={() =>
                        void router.navigate({
                          to: '/projects/$id',
                          params: { id: String(project.id) },
                        })
                      }
                    >
                      {project.name}
                    </button>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <span className="text-muted-foreground font-mono text-xs">
                        {publicUrl(project.slug)}
                      </span>
                      <CopyButton
                        text={publicUrl(project.slug)}
                        ariaLabel={t('list.copy_url_aria')}
                      />
                    </div>
                  </TableCell>
                  <TableCell>
                    {project.code_enabled ? (
                      <Badge className="bg-green-600 text-white hover:bg-green-600">
                        {t('list.badge_code_on')}
                      </Badge>
                    ) : (
                      <Badge className="bg-amber-500 text-white hover:bg-amber-500">
                        {t('list.badge_free')}
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    {/* NOTE: `active_version_id` is a DB primary key, NOT the
                        sequential version number (n). Rendering `v{id}` would show
                        a misleading number to users (e.g. "v37" for version n=2).
                        We therefore show a neutral "Deployed" indicator when a
                        version is active, and "—" otherwise.
                        BACKLOG: backend should enrich the list DTO with
                        `active_version_n` (the sequential n) + `version_count`
                        so the column can display "v2 / 3 versions" as per §7. */}
                    {project.active_version_id != null
                      ? t('list.deployed')
                      : t('common.dash')}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </main>

      <ProjectForm
        open={formOpen}
        mode="create"
        onOpenChange={setFormOpen}
      />
    </div>
  )
}
