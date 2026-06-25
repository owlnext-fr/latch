import { useState } from 'react'
import type { ReactNode } from 'react'
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

  let content: ReactNode
  if (isLoading) {
    content = <p className="text-muted-foreground text-sm">{t('common.loading')}</p>
  } else if (!projects || projects.length === 0) {
    content = (
      <div className="flex flex-col items-center gap-4 py-16">
        <p className="text-muted-foreground">{t('list.empty')}</p>
        <Button type="button" onClick={() => setFormOpen(true)}>
          {t('list.create_first')}
        </Button>
      </div>
    )
  } else {
    content = (
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
                  onClick={() => {
                    router.navigate({
                      to: '/projects/$id',
                      params: { id: String(project.id) },
                    })
                  }}
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
                {/* `active_version_n` = numéro de version (n), pas le PK.
                    Affiche "v{n} · {count} versions", ou "—" si aucun déploiement. */}
                {project.active_version_n == null ? (
                  <span className="text-muted-foreground">{t('common.dash')}</span>
                ) : (
                  <span className="flex items-baseline gap-2">
                    <span className="font-medium">{`v${project.active_version_n}`}</span>
                    <span className="text-muted-foreground text-xs">
                      {t('list.versions_count', { count: project.version_count })}
                    </span>
                  </span>
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    )
  }

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

        {content}
      </main>

      <ProjectForm
        open={formOpen}
        mode="create"
        onOpenChange={setFormOpen}
      />
    </div>
  )
}
