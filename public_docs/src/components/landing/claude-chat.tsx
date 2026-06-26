import { FileCode2, Check } from 'lucide-react';
import { LatchLogo } from '@/components/logo';

/** Avatar Claude (étincelle stylisée) — pas de marque externe. */
function ClaudeAvatar() {
  return (
    <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-fd-primary text-fd-primary-foreground">
      <svg viewBox="0 0 24 24" fill="currentColor" className="size-4" aria-hidden="true">
        <path d="M12 2l1.9 5.8a3 3 0 0 0 1.9 1.9L21.6 12l-5.8 1.9a3 3 0 0 0-1.9 1.9L12 21.6l-1.9-5.8a3 3 0 0 0-1.9-1.9L2.4 12l5.8-1.9a3 3 0 0 0 1.9-1.9L12 2.4z" />
      </svg>
    </div>
  );
}

function UserAvatar() {
  return (
    <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-fd-muted text-xs font-semibold text-fd-muted-foreground">
      You
    </div>
  );
}

function Bubble({
  from,
  children,
}: {
  from: 'claude' | 'user';
  children: React.ReactNode;
}) {
  const isUser = from === 'user';
  return (
    <div className={`flex items-start gap-2.5 ${isUser ? 'flex-row-reverse' : ''}`}>
      {isUser ? <UserAvatar /> : <ClaudeAvatar />}
      <div
        className={`max-w-[78%] rounded-2xl px-3.5 py-2 text-sm ${
          isUser
            ? 'rounded-tr-sm bg-fd-primary text-fd-primary-foreground'
            : 'rounded-tl-sm bg-fd-muted text-fd-foreground'
        }`}
      >
        {children}
      </div>
    </div>
  );
}

/**
 * Conversation Claude simulée (CSS uniquement, statique) : illustre le flux
 * « publish from Claude » sans capturer claude.ai. Données fictives (placeholders).
 */
export function ClaudeChat() {
  return (
    <div className="w-full overflow-hidden rounded-xl border border-fd-border bg-fd-card shadow-sm">
      {/* chrome de fenêtre */}
      <div className="flex items-center gap-2 border-b border-fd-border px-4 py-2.5">
        <div className="flex gap-1.5">
          <span className="size-3 rounded-full bg-fd-border" />
          <span className="size-3 rounded-full bg-fd-border" />
          <span className="size-3 rounded-full bg-fd-border" />
        </div>
        <span className="ml-2 inline-flex items-center gap-1.5 text-xs text-fd-muted-foreground">
          <LatchLogo className="size-3.5" /> latch · MCP
        </span>
      </div>

      {/* fil de conversation */}
      <div className="flex flex-col gap-3 p-4">
        <Bubble from="claude">
          Here&apos;s your prototype — a single-file HTML landing page.
          <span className="mt-2 flex w-fit items-center gap-1.5 rounded-md border border-fd-border bg-fd-background px-2 py-1 text-xs text-fd-muted-foreground">
            <FileCode2 className="size-3.5" /> landing.html
          </span>
        </Bubble>

        <Bubble from="user">Nice. Can you publish it to latch?</Bubble>

        <Bubble from="claude">Sure — what&apos;s your deploy token?</Bubble>

        <Bubble from="user">
          <code className="font-mono tracking-wider">••••••••••••••••</code>
        </Bubble>

        <Bubble from="claude">
          <span className="inline-flex items-center gap-1.5 font-medium">
            <Check className="size-4" /> Published version 2
          </span>
          <span className="mt-1.5 block text-xs text-fd-muted-foreground">
            Live at{' '}
            <span className="font-mono text-fd-foreground">
              https://latch.mycompany.com/c/my-project-k7Qp2maZ
            </span>{' '}
            — PIN protected.
          </span>
        </Bubble>
      </div>
    </div>
  );
}
