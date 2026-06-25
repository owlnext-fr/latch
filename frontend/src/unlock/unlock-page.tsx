import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { InputOTP, InputOTPGroup, InputOTPSlot } from '@/components/ui/input-otp'
import { REGEXP_ONLY_DIGITS } from 'input-otp'
import { reloadPage } from './reload'

function slugFromPath(): string {
  // /c/<slug> → segment d'indice 1
  return globalThis.location.pathname.split('/').filter(Boolean)[1] ?? ''
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

  async function doUnlock() {
    if (busy) return
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
      if (res.status === 429) {
        setError(t('unlock.error_throttled'))
      } else if (res.status === 401) {
        setPin('')
        setError(t('unlock.error_wrong'))
      } else {
        setError(t('unlock.error_generic'))
      }
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
          <CardDescription>{t('unlock.instructions')}</CardDescription>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              void doUnlock()
            }}
            className="flex flex-col gap-4"
          >
            <div className="flex flex-col items-center gap-4">
              <Label htmlFor="pin">{t('unlock.pin_label')}</Label>
              <InputOTP
                id="pin"
                maxLength={6}
                pattern={REGEXP_ONLY_DIGITS}
                value={pin}
                onChange={(v) => {
                  setPin(v)
                  if (error) setError(null)
                }}
                onComplete={() => void doUnlock()}
                aria-invalid={error ? true : undefined}
              >
                <InputOTPGroup>
                  <InputOTPSlot index={0} aria-invalid={error ? true : undefined} />
                  <InputOTPSlot index={1} aria-invalid={error ? true : undefined} />
                  <InputOTPSlot index={2} aria-invalid={error ? true : undefined} />
                  <InputOTPSlot index={3} aria-invalid={error ? true : undefined} />
                  <InputOTPSlot index={4} aria-invalid={error ? true : undefined} />
                  <InputOTPSlot index={5} aria-invalid={error ? true : undefined} />
                </InputOTPGroup>
              </InputOTP>
            </div>
            {error && (
              <p role="alert" className="text-center text-sm text-destructive">
                {error}
              </p>
            )}
            <Button type="submit" loading={busy} disabled={pin.length < 6}>
              {t('unlock.submit')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
