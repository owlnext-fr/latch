import logoUrl from '@/assets/latch-logo.svg'

export function Logo({ className }: Readonly<{ className?: string }>) {
  return <img src={logoUrl} alt="latch" className={className} />
}
