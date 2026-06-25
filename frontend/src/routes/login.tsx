import { useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { LocaleSwitcher } from '@/components/locale-switcher'
import { useLogin } from '@/hooks/use-auth'

const loginSchema = z.object({
  user: z.string().min(1),
  pass: z.string().min(1),
})

type LoginForm = z.infer<typeof loginSchema>

export function LoginPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { mutate, isPending } = useLogin()
  const [error, setError] = useState<string | null>(null)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginForm>({ resolver: zodResolver(loginSchema) })

  function onSubmit(data: LoginForm) {
    setError(null)
    mutate(
      { user: data.user, pass: data.pass },
      {
        onSuccess: () => {
          void navigate({ to: '/' })
        },
        onError: () => {
          setError(t('login.error_invalid'))
        },
      },
    )
  }

  return (
    <div className="relative grid min-h-screen place-items-center">
      {/* Locale switcher in the top-right corner */}
      <div className="absolute top-4 right-4">
        <LocaleSwitcher />
      </div>

      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>{t('login.title')}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="login-user"
                className="text-sm font-medium leading-none"
              >
                {t('login.user')}
              </label>
              <input
                id="login-user"
                type="text"
                autoComplete="username"
                aria-invalid={!!errors.user}
                aria-describedby={errors.user ? 'login-user-error' : undefined}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50"
                {...register('user')}
              />
              {errors.user && (
                <span id="login-user-error" className="text-xs text-destructive" role="alert">
                  {errors.user.message}
                </span>
              )}
            </div>

            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="login-pass"
                className="text-sm font-medium leading-none"
              >
                {t('login.pass')}
              </label>
              <input
                id="login-pass"
                type="password"
                autoComplete="current-password"
                aria-invalid={!!errors.pass}
                aria-describedby={errors.pass ? 'login-pass-error' : undefined}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50"
                {...register('pass')}
              />
              {errors.pass && (
                <span id="login-pass-error" className="text-xs text-destructive" role="alert">
                  {errors.pass.message}
                </span>
              )}
            </div>

            {error && (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            )}

            <Button type="submit" loading={isPending}>
              {t('login.submit')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
