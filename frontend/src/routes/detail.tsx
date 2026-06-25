import { useState } from 'react'
import { useParams, useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { Zap } from 'lucide-react'
import { Topbar } from '@/components/topbar'
import { CopyButton } from '@/components/copy-button'
import { PinField } from '@/components/pin-field'
import { ProjectForm } from '@/components/project-form'
import { DeployPanel } from '@/components/deploy-panel'
import { DeleteProjectPanel } from '@/components/delete-project-panel'
import { DeleteVersionPanel } from '@/components/delete-version-panel'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { useProject, useActivateVersion } from '@/hooks/use-projects'
import { publicUrl } from '@/lib/utils'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

function previewUrl(projectId: number, n: number): string {
  return `/api/projects/${projectId}/versions/${n}/preview`
}

export function DetailPage() {
  const { t } = useTranslation()
  const router = useRouter()
  const { id: idStr } = useParams({ strict: false }) as { id?: string }
  const id = Number(idStr ?? '0')

  const { data: project, isLoading, isError } = useProject(id)
  const activateVersion = useActivateVersion()

  const [editOpen, setEditOpen] = useState(false)
  const [deployOpen, setDeployOpen] = useState(false)
  const [deleteProjectOpen, setDeleteProjectOpen] = useState(false)
  const [deleteVersion, setDeleteVersion] = useState<VersionItem | null>(null)

  return (
    <div className="flex min-h-screen flex-col">
      <Topbar />

      <main className="flex-1 p-6">
        {/* Breadcrumb */}
        <div className="mb-4">
          <button
            type="button"
            className="text-sm text-muted-foreground hover:text-foreground"
            onClick={() => void router.navigate({ to: '/' })}
          >
            {t('detail.back')}
          </button>
        </div>

        {isLoading ? (
          <p className="text-muted-foreground text-sm">{t('common.loading')}</p>
        ) : isError || !project ? (
          <p className="text-destructive text-sm">
            {t('error.server', { code: '' })}
          </p>
        ) : (
          <>
            {/* Page header + actions */}
            <div className="mb-6 flex items-start justify-between gap-4">
              <div>
                <h1 className="font-heading text-xl font-semibold">
                  {project.name}
                </h1>
                <p className="text-muted-foreground text-sm">
                  {t('detail.intro')}
                </p>
              </div>

              <div className="flex shrink-0 items-center gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setEditOpen(true)}
                >
                  {t('common.edit')}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  onClick={() => setDeployOpen(true)}
                >
                  {t('common.deploy')}
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  onClick={() => setDeleteProjectOpen(true)}
                >
                  {t('common.delete')}
                </Button>
              </div>
            </div>

            <div className="grid gap-4 lg:grid-cols-2">
              {/* Public access card */}
              <Card>
                <CardHeader>
                  <CardTitle>{t('detail.access_title')}</CardTitle>
                </CardHeader>
                <CardContent className="flex flex-col gap-3">
                  <div>
                    <p className="mb-1 text-xs font-medium text-muted-foreground">
                      {t('detail.url_label')}
                    </p>
                    <div className="flex items-center gap-1">
                      <span className="font-mono text-xs text-muted-foreground">
                        {publicUrl(project.slug)}
                      </span>
                      <CopyButton
                        text={publicUrl(project.slug)}
                        ariaLabel={t('detail.copy_url_aria')}
                      />
                    </div>
                  </div>

                  <div>
                    {project.code_enabled ? (
                      <>
                        <p className="mb-1 text-xs font-medium text-muted-foreground">
                          {t('detail.code_label')}
                        </p>
                        <PinField pin={project.pin ?? null} />
                      </>
                    ) : (
                      <p className="text-sm text-muted-foreground">
                        {t('detail.free_access')}
                      </p>
                    )}
                  </div>
                </CardContent>
              </Card>

              {/* Configuration card */}
              <Card>
                <CardHeader>
                  <CardTitle>{t('detail.config_title')}</CardTitle>
                </CardHeader>
                <CardContent className="flex flex-col gap-3">
                  <div>
                    <p className="mb-0.5 text-xs font-medium text-muted-foreground">
                      {t('detail.brand_label')}
                    </p>
                    <p className="text-sm">
                      {project.brand_name ?? t('common.dash')}
                    </p>
                  </div>
                  <div>
                    <p className="mb-0.5 text-xs font-medium text-muted-foreground">
                      {t('detail.code_label')}
                    </p>
                    <p className="text-sm">
                      {project.code_enabled
                        ? t('detail.code_on')
                        : t('detail.code_off')}
                    </p>
                  </div>
                </CardContent>
              </Card>
            </div>

            {/* Versions card */}
            <Card className="mt-4">
              <CardHeader>
                <CardTitle>{t('detail.versions_title')}</CardTitle>
              </CardHeader>
              <CardContent>
                {project.versions.length === 0 ? (
                  <div className="flex flex-col items-center gap-3 py-8">
                    <Zap className="h-8 w-8 text-muted-foreground" />
                    <p className="text-center text-muted-foreground">
                      {t('deploy.dropzone_idle')}
                    </p>
                    <Button
                      type="button"
                      onClick={() => setDeployOpen(true)}
                    >
                      {t('common.deploy')}
                    </Button>
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t('detail.col_num')}</TableHead>
                        <TableHead>{t('detail.col_date')}</TableHead>
                        <TableHead>{t('detail.col_status')}</TableHead>
                        <TableHead />
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {project.versions.map((v) => (
                        <TableRow key={v.id}>
                          <TableCell>{v.n}</TableCell>
                          <TableCell>
                            {new Date(v.created_at).toLocaleDateString()}
                          </TableCell>
                          <TableCell>
                            {v.is_active && (
                              <Badge className="bg-green-600 text-white hover:bg-green-600">
                                {t('common.active')}
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center justify-end gap-1">
                              {/* Activate button (hidden if already active) */}
                              {!v.is_active && (
                                <Button
                                  type="button"
                                  variant="ghost"
                                  size="sm"
                                  aria-label={t('detail.activate_aria')}
                                  onClick={() =>
                                    activateVersion.mutate({ id, n: v.n })
                                  }
                                >
                                  {t('detail.activate_aria')}
                                </Button>
                              )}

                              {/* Preview link */}
                              <a
                                href={previewUrl(id, v.n)}
                                target="_blank"
                                rel="noopener noreferrer"
                                aria-label={t('detail.preview_aria')}
                                className="inline-flex h-8 items-center rounded-md px-3 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                              >
                                {t('detail.preview_aria')}
                              </a>

                              {/* Delete button (hidden if active) */}
                              {!v.is_active && (
                                <Button
                                  type="button"
                                  variant="ghost"
                                  size="sm"
                                  aria-label={t('detail.delete_aria')}
                                  className="text-destructive hover:text-destructive"
                                  onClick={() => setDeleteVersion(v)}
                                >
                                  {t('detail.delete_aria')}
                                </Button>
                              )}
                            </div>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </CardContent>
            </Card>

            {/* Panels */}
            <ProjectForm
              open={editOpen}
              mode="edit"
              project={project}
              onOpenChange={setEditOpen}
            />

            <DeployPanel
              projectId={id}
              open={deployOpen}
              onOpenChange={setDeployOpen}
            />

            <DeleteProjectPanel
              project={project}
              open={deleteProjectOpen}
              onOpenChange={setDeleteProjectOpen}
            />

            {deleteVersion && (
              <DeleteVersionPanel
                projectId={id}
                version={deleteVersion}
                open={deleteVersion !== null}
                onOpenChange={(isOpen) => {
                  if (!isOpen) setDeleteVersion(null)
                }}
              />
            )}
          </>
        )}
      </main>
    </div>
  )
}
