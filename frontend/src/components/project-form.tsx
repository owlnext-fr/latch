import { useEffect } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { PinField } from '@/components/pin-field'
import {
  useClearCode,
  useCreateProject,
  useSetCode,
  useUpdateProject,
} from '@/hooks/use-projects'
import type { components } from '@/api/schema'

type ProjectDetail = components['schemas']['ProjectDetail']

interface ProjectFormProps {
  open: boolean
  mode: 'create' | 'edit'
  project?: ProjectDetail
  onOpenChange: (open: boolean) => void
}

/**
 * Generate a fresh 6-digit PIN (string).
 * Uses the Web Crypto CSPRNG rather than `Math.random()` — the modulo bias on a
 * 6-digit code is negligible, and the real brute-force defense is the rate-limit
 * on `/c/<slug>/unlock` (contrat §9.5), not the PIN entropy.
 */
function generatePin(): string {
  const buf = new Uint32Array(6)
  crypto.getRandomValues(buf)
  let pin = ''
  for (let i = 0; i < 6; i += 1) {
    pin += String(buf[i] % 10)
  }
  return pin
}

interface FormValues {
  name: string
  code_enabled: boolean
  pin: string
  brand_name: string
}

export function ProjectForm({
  open,
  mode,
  project,
  onOpenChange,
}: ProjectFormProps) {
  const { t } = useTranslation()
  const createProject = useCreateProject()
  const updateProject = useUpdateProject()
  const setCode = useSetCode()
  const clearCode = useClearCode()

  const schema = z
    .object({
      name: z.string().trim().min(1, { message: t('form.err_name') }),
      code_enabled: z.boolean(),
      pin: z.string(),
      brand_name: z.string(),
    })
    .refine((data) => !data.code_enabled || /^\d{6}$/.test(data.pin), {
      message: t('form.err_pin'),
      path: ['pin'],
    })

  const {
    register,
    handleSubmit,
    reset,
    setValue,
    control,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: '',
      code_enabled: true,
      pin: '',
      brand_name: '',
    },
  })

  // Reset all fields on each (re)open — create vs edit seed.
  useEffect(() => {
    if (!open) return
    if (mode === 'edit' && project) {
      reset({
        name: project.name,
        code_enabled: project.code_enabled,
        pin: project.pin ?? generatePin(),
        brand_name: project.brand_name ?? '',
      })
    } else {
      reset({
        name: '',
        code_enabled: true,
        pin: generatePin(),
        brand_name: '',
      })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, mode, project])

  const codeEnabled = useWatch({ control, name: 'code_enabled' }) ?? true
  const pin = useWatch({ control, name: 'pin' }) ?? ''

  const onSubmit = handleSubmit((values) => {
    const trimmedBrand = values.brand_name.trim()
    if (mode === 'create') {
      createProject.mutate(
        {
          name: values.name.trim(),
          code_enabled: values.code_enabled,
          pin: values.code_enabled ? values.pin : null,
          brand_name: trimmedBrand === '' ? null : trimmedBrand,
        },
        { onSuccess: () => onOpenChange(false) },
      )
      return
    }

    if (!project) return
    const id = project.id
    const wasEnabled = project.code_enabled
    updateProject.mutate(
      {
        id,
        body: {
          name: values.name.trim(),
          brand_name: trimmedBrand === '' ? null : trimmedBrand,
        },
      },
      {
        onSuccess: () => {
          if (values.code_enabled && (!wasEnabled || values.pin !== project.pin)) {
            setCode.mutate({ id, pin: values.pin })
          } else if (!values.code_enabled && wasEnabled) {
            clearCode.mutate(id)
          }
          onOpenChange(false)
        },
      },
    )
  })

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>
            {mode === 'create' ? t('form.title_create') : t('form.title_edit')}
          </SheetTitle>
        </SheetHeader>

        <form onSubmit={onSubmit} className="flex flex-1 flex-col gap-5 p-4">
          {/* Name */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="project-name">{t('form.name')}</Label>
            <Input id="project-name" {...register('name')} />
            <p className="text-muted-foreground text-xs">{t('form.name_help')}</p>
            {errors.name && (
              <p className="text-destructive text-xs">{errors.name.message}</p>
            )}
          </div>

          {/* Slug (read-only) */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="project-slug">{t('form.slug')}</Label>
            <Input
              id="project-slug"
              readOnly
              disabled={mode === 'edit'}
              value={mode === 'edit' && project ? project.slug : ''}
              placeholder={t('common.dash')}
            />
            <p className="text-muted-foreground text-xs">{t('form.slug_help')}</p>
          </div>

          {/* Code enabled */}
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label htmlFor="project-code">{t('form.code')}</Label>
              <Switch
                id="project-code"
                checked={codeEnabled}
                onCheckedChange={(checked) =>
                  setValue('code_enabled', checked, { shouldValidate: true })
                }
              />
            </div>
            <p className="text-muted-foreground text-xs">{t('form.code_help')}</p>
          </div>

          {/* PIN — always rendered, disabled when code off */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="project-pin">{t('form.pin')}</Label>
            <div className="flex items-center gap-2">
              <PinField
                pin={pin}
                editable
                disabled={!codeEnabled}
                onChange={(value) =>
                  setValue('pin', value, { shouldValidate: true })
                }
              />
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={!codeEnabled}
                onClick={() =>
                  setValue('pin', generatePin(), { shouldValidate: true })
                }
              >
                {t('common.regenerate')}
              </Button>
            </div>
            <p className="text-muted-foreground text-xs">{t('form.pin_help')}</p>
            {errors.pin && (
              <p className="text-destructive text-xs">{errors.pin.message}</p>
            )}
          </div>

          {/* Brand name (optional) */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="project-brand">{t('form.brand')}</Label>
            <Input id="project-brand" {...register('brand_name')} />
            <p className="text-muted-foreground text-xs">{t('form.brand_help')}</p>
          </div>

          <SheetFooter className="flex-row justify-end gap-2 px-0">
            <Button
              type="button"
              variant="ghost"
              onClick={() => onOpenChange(false)}
            >
              {t('common.cancel')}
            </Button>
            <Button
              type="submit"
              loading={
                createProject.isPending ||
                updateProject.isPending ||
                setCode.isPending ||
                clearCode.isPending
              }
            >
              {t('common.save')}
            </Button>
          </SheetFooter>
        </form>
      </SheetContent>
    </Sheet>
  )
}
