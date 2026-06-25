import { useRef, useState } from 'react'
import type { ChangeEvent, DragEvent, FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { useDeploy } from '@/hooks/use-projects'
import { humanSize } from '@/lib/utils'

interface DeployPanelProps {
  projectId: number
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Inner content of the deploy panel — unmounted/remounted when the sheet
 * opens so state is always fresh. Rendered only when `open` is true.
 */
function DeployPanelContent({
  projectId,
  onOpenChange,
}: Readonly<{
  projectId: number
  onOpenChange: (open: boolean) => void
}>) {
  const { t } = useTranslation()
  const deploy = useDeploy()

  const [file, setFile] = useState<File | null>(null)
  const [activate, setActivate] = useState(true)
  const [isDragOver, setIsDragOver] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const inputRef = useRef<HTMLInputElement>(null)

  function acceptFile(f: File) {
    setFile(f)
    setError(null)
  }

  function handleDragOver(e: DragEvent<HTMLButtonElement>) {
    e.preventDefault()
    setIsDragOver(true)
  }

  function handleDragLeave() {
    setIsDragOver(false)
  }

  function handleDrop(e: DragEvent<HTMLButtonElement>) {
    e.preventDefault()
    setIsDragOver(false)
    const dropped = e.dataTransfer.files[0]
    if (dropped) acceptFile(dropped)
  }

  function handleInputChange(e: ChangeEvent<HTMLInputElement>) {
    const chosen = e.target.files?.[0]
    if (chosen) acceptFile(chosen)
    // Reset input so the same file can be re-picked
    e.target.value = ''
  }

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()

    if (!file) {
      setError(t('deploy.err_no_file'))
      return
    }

    let html: string
    try {
      html = await file.text()
    } catch {
      setError(t('deploy.err_read'))
      return
    }

    deploy.mutate(
      { id: projectId, body: { html, activate } },
      { onSuccess: () => onOpenChange(false) },
    )
  }

  function computeDropzoneText() {
    if (file) return t('deploy.file_chosen', { name: file.name, size: humanSize(file.size) })
    if (isDragOver) return t('deploy.dropzone_hover')
    return t('deploy.dropzone_idle')
  }
  const dropzoneText = computeDropzoneText()

  let dropzoneBorder: string
  if (isDragOver) dropzoneBorder = 'border-primary bg-primary/5 text-primary'
  else if (file) dropzoneBorder = 'border-green-500 bg-green-50 text-green-700'
  else dropzoneBorder = 'border-input text-muted-foreground hover:border-primary/50 hover:text-foreground'

  return (
    <form
      onSubmit={(e) => {
        handleSubmit(e)
      }}
      className="flex flex-1 flex-col gap-5 p-4"
    >
      {/* Dropzone */}
      <div className="flex flex-col gap-1.5">
        <Label>{t('deploy.file')}</Label>

        {/* Hidden file input */}
        <input
          ref={inputRef}
          type="file"
          accept="text/html,.html"
          className="sr-only"
          tabIndex={-1}
          aria-hidden="true"
          onChange={handleInputChange}
        />

        {/* Clickable drop zone — keyboard operable via button */}
        <button
          type="button"
          onClick={() => inputRef.current?.click()}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          className={[
            'flex min-h-[120px] w-full cursor-pointer items-center justify-center rounded-lg border-2 border-dashed px-4 py-6 text-sm transition-colors',
            dropzoneBorder,
          ].join(' ')}
          aria-label={t('deploy.dropzone_idle')}
        >
          {dropzoneText}
        </button>

        {error && <p className="text-destructive text-xs">{error}</p>}
      </div>

      {/* Activate checkbox */}
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center gap-2">
          <input
            id="deploy-activate"
            type="checkbox"
            checked={activate}
            onChange={(e) => setActivate(e.target.checked)}
            className="h-4 w-4 rounded border-input"
          />
          <Label htmlFor="deploy-activate">{t('deploy.activate')}</Label>
        </div>
        <p className="text-muted-foreground pl-6 text-xs">
          {t('deploy.activate_help')}
        </p>
      </div>

      <SheetFooter className="flex-row justify-end gap-2 px-0">
        <Button
          type="button"
          variant="ghost"
          onClick={() => onOpenChange(false)}
        >
          {t('common.cancel')}
        </Button>
        <Button type="submit" loading={deploy.isPending}>
          {t('deploy.btn')}
        </Button>
      </SheetFooter>
    </form>
  )
}

export function DeployPanel({ projectId, open, onOpenChange }: Readonly<DeployPanelProps>) {
  const { t } = useTranslation()

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>{t('deploy.title')}</SheetTitle>
        </SheetHeader>

        {/* Mount/unmount the content so state is always fresh on open */}
        {open && (
          <DeployPanelContent
            projectId={projectId}
            onOpenChange={onOpenChange}
          />
        )}
      </SheetContent>
    </Sheet>
  )
}
