import { useTranslation } from 'react-i18next'
import { Topbar } from '@/components/topbar'
import { CopyButton } from '@/components/copy-button'
import { PinField } from '@/components/pin-field'
import { useSettings } from '@/hooks/use-settings'

export function SettingsPage() {
  const { t } = useTranslation()
  const { data, isLoading, isError } = useSettings()

  return (
    <div className="min-h-screen">
      <Topbar />
      <main className="mx-auto max-w-2xl px-4 py-8">
        <h1 className="text-xl font-bold">{t('settings.title')}</h1>
        <p className="text-muted-foreground mt-1 text-sm">{t('settings.mcp_intro')}</p>

        {isLoading ? (
          <p className="text-muted-foreground mt-6 text-sm">{t('common.loading')}</p>
        ) : isError ? (
          <p className="text-destructive mt-6 text-sm">{t('error.network')}</p>
        ) : data ? (
          <dl className="mt-6 space-y-6">
            <div>
              <dt className="text-sm font-medium">{t('settings.mcp_url')}</dt>
              <dd className="mt-1 flex items-center gap-2">
                <span className="font-mono text-sm">{data.mcp_url}</span>
                <CopyButton text={data.mcp_url} ariaLabel={t('settings.copy_mcp_url')} />
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium">{t('settings.deploy_token')}</dt>
              <dd className="mt-1">
                <PinField pin={data.deploy_token} />
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium">{t('settings.public_base_url')}</dt>
              <dd className="mt-1 font-mono text-sm">{data.public_base_url}</dd>
            </div>
          </dl>
        ) : null}
      </main>
    </div>
  )
}
