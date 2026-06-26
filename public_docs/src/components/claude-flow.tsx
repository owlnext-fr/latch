import { Sparkles, Server, FileCode2, Link2, ArrowRight } from 'lucide-react';

const nodes = [
  { icon: Sparkles, title: 'Claude', sub: 'asks to publish' },
  { icon: Server, title: 'POST /mcp', sub: 'deploy_token checked first' },
  { icon: FileCode2, title: 'deploy_prototype', sub: 'new version, activated' },
  { icon: Link2, title: 'Client URL', sub: '/c/<slug> returned' },
];

/**
 * Schéma annoté du flux « publish from Claude » — composant themeable (fd-*),
 * suit le thème clair/sombre et reste responsive (vs un SVG statique).
 */
export function ClaudeFlow() {
  return (
    <div className="my-6 flex flex-col items-stretch gap-2 sm:flex-row sm:items-center">
      {nodes.map((n, i) => (
        <div key={n.title} className="flex flex-col items-stretch gap-2 sm:flex-row sm:items-center">
          <div className="flex flex-1 items-center gap-3 rounded-lg border border-fd-border bg-fd-card p-3">
            <n.icon className="size-5 shrink-0 text-fd-primary" />
            <div className="leading-tight">
              <div className="text-sm font-medium">{n.title}</div>
              <div className="text-xs text-fd-muted-foreground">{n.sub}</div>
            </div>
          </div>
          {i < nodes.length - 1 && (
            <ArrowRight className="mx-auto size-4 shrink-0 rotate-90 text-fd-muted-foreground sm:rotate-0" />
          )}
        </div>
      ))}
    </div>
  );
}
