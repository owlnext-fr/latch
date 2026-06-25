import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { reloadPage } from './reload'

function slugFromPath(): string {
  // /c/<slug> → segment d'indice 1
  return window.location.pathname.split('/').filter(Boolean)[1] ?? ''
}

export function UnlockPage() {
  const { t } = useTranslation()
  const slug = slugFromPath()
  const [brand, setBrand] = useState<string | null>(null)
  const [pin, setPin] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const ac = new AbortController()
    fetch(`/api/public/${slug}`, { signal: ac.signal })
      .then((r) => (r.ok ? r.json() : null))
      .then((meta) => meta && setBrand(meta.brand_name ?? null))
      .catch(() => {})
    return () => ac.abort()
  }, [slug])

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    setError(null)
    setBusy(true)
    try {
      const res = await fetch(`/c/${slug}/unlock`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pin }),
      })
      if (res.status === 204) {
        reloadPage()
        return
      }
      if (res.status === 429) setError(t('unlock.error_throttled'))
      else if (res.status === 401) setError(t('unlock.error_wrong'))
      else setError(t('unlock.error_generic'))
    } catch {
      setError(t('unlock.error_generic'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-background p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>
            {brand ? t('unlock.title_brand', { brand }) : t('unlock.title_neutral')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="pin">{t('unlock.pin_label')}</Label>
              <Input
                id="pin"
                inputMode="numeric"
                autoComplete="off"
                maxLength={6}
                value={pin}
                onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))}
                aria-invalid={error ? true : undefined}
              />
            </div>
            {error && (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            )}
            <Button type="submit" disabled={busy || pin.length === 0}>
              {t('unlock.submit')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
