import { useTranslation } from 'react-i18next'
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { CopyButton } from '@/components/copy-button'
import { PinField } from '@/components/pin-field'
import { LanguageSelect } from '@/components/language-select'
import { ThemeToggle } from '@/components/theme-toggle'
import { useSettings } from '@/hooks/use-settings'

interface SettingsSheetProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SettingsSheet({ open, onOpenChange }: Readonly<SettingsSheetProps>) {
  const { t } = useTranslation()
  const { data, isLoading, isError } = useSettings(open)

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>{t('settings.title')}</SheetTitle>
        </SheetHeader>

        <div className="flex flex-1 flex-col gap-6 p-4">
          <section className="flex flex-col gap-4">
            <h3 className="text-muted-foreground text-xs font-medium uppercase">
              {t('settings.section_mcp')}
            </h3>

            {isLoading ? (
              <p className="text-muted-foreground text-sm">{t('common.loading')}</p>
            ) : isError ? (
              <p className="text-destructive text-sm">{t('error.network')}</p>
            ) : data ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.mcp_url')}</span>
                  <span className="flex items-center gap-2">
                    <span className="font-mono text-sm break-all">{data.mcp_url}</span>
                    <CopyButton text={data.mcp_url} ariaLabel={t('settings.copy_mcp_url')} />
                  </span>
                  <p className="text-muted-foreground text-xs">{t('settings.mcp_url_help')}</p>
                </div>

                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.deploy_token')}</span>
                  <PinField pin={data.deploy_token} />
                  <p className="text-muted-foreground text-xs">{t('settings.deploy_token_help')}</p>
                </div>

                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.public_base_url')}</span>
                  <span className="font-mono text-sm break-all">{data.public_base_url}</span>
                  <p className="text-muted-foreground text-xs">{t('settings.public_base_url_help')}</p>
                </div>
              </>
            ) : null}
          </section>

          <section className="flex flex-col gap-4">
            <h3 className="text-muted-foreground text-xs font-medium uppercase">
              {t('settings.section_preferences')}
            </h3>

            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">{t('settings.language')}</span>
              <LanguageSelect />
              <p className="text-muted-foreground text-xs">{t('settings.language_help')}</p>
            </div>

            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">{t('settings.theme')}</span>
              <ThemeToggle />
              <p className="text-muted-foreground text-xs">{t('settings.theme_help')}</p>
            </div>
          </section>
        </div>
      </SheetContent>
    </Sheet>
  )
}
