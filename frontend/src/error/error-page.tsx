import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Logo } from '@/components/logo'
import { useDocumentTitle } from '@/hooks/use-document-title'

export function ErrorPage() {
  const { t } = useTranslation()
  useDocumentTitle(t('error.page_title'))

  return (
    <div className="flex min-h-svh items-center justify-center bg-background p-4">
      <div className="flex w-full max-w-sm flex-col items-center gap-6">
        <Logo className="size-12" />
        <Card className="w-full">
          <CardHeader>
            <CardTitle>{t('error.title')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground text-sm">{t('error.message')}</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
