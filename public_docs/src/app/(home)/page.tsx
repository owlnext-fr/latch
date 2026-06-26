import Link from 'next/link';
import type { ReactNode } from 'react';
import {
  Layers,
  Sparkles,
  KeyRound,
  ShieldCheck,
  Box,
  Scale,
  ArrowRight,
} from 'lucide-react';
import { LatchLogo } from '@/components/logo';
import { GithubIcon } from '@/components/github-icon';
import { ClaudeChat } from '@/components/landing/claude-chat';
import { gitConfig } from '@/lib/shared';

const githubUrl = `https://github.com/${gitConfig.user}/${gitConfig.repo}`;

const steps: { n: number; title: string; body: string; visual: ReactNode }[] = [
  {
    n: 1,
    title: 'Create your project',
    body: 'In the admin, spin up a project in a side-panel. You get a non-guessable slug and an optional 6-digit PIN — code protection is on by default.',
    visual: (
      // eslint-disable-next-line @next/next/no-img-element
      <img src="/img/admin-list.png" alt="latch admin — project list" className="h-auto w-full" />
    ),
  },
  {
    n: 2,
    title: 'Publish from Claude',
    body: 'Connect the MCP endpoint once, then just ask. Claude deploys the prototype through the deploy_prototype tool and hands you back the live URL — versioned automatically.',
    visual: <ClaudeChat />,
  },
  {
    n: 3,
    title: 'Share the link',
    body: 'Send the client a single stable link. Protected projects show a styled unlock page; once the PIN is entered, the active version is served — always the latest.',
    visual: (
      // eslint-disable-next-line @next/next/no-img-element
      <img src="/img/unlock.png" alt="latch unlock page" className="h-auto w-full" />
    ),
  },
];

const features = [
  {
    icon: Layers,
    title: 'Three surfaces, one binary',
    body: 'Client serving (/c), a React admin, and an MCP endpoint — all on a single Loco (axum) binary.',
  },
  {
    icon: Sparkles,
    title: 'Publish from Claude',
    body: 'Deploy a prototype straight from Claude through the Model Context Protocol. No copy-pasting files.',
  },
  {
    icon: KeyRound,
    title: 'Optional access codes',
    body: 'Protect any project with a 6-digit PIN and a styled unlock page. Rotate the code to revoke access.',
  },
  {
    icon: ShieldCheck,
    title: 'Fail-secure by default',
    body: 'Boot refuses to start in production without its required secrets. Rate-limits guard unlock and login.',
  },
  {
    icon: Box,
    title: 'Single Rust binary',
    body: 'Distroless image, SQLite bundled in. No Redis, no workers, no external services to run.',
  },
  {
    icon: Scale,
    title: 'Free & open source',
    body: 'Dual-licensed MIT or Apache-2.0. Public image on GHCR — no login to pull.',
  },
];

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      {/* 1. Hero */}
      <section className="mx-auto flex w-full max-w-5xl flex-col items-center px-6 pt-20 pb-16 text-center">
        <LatchLogo className="size-16" />
        <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-5xl">latch</h1>
        <p className="mt-4 max-w-2xl text-lg text-fd-muted-foreground">
          Serve single-file HTML prototypes behind a controlled host — with versioning and
          optional per-project access codes.
        </p>
        <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/docs/quickstart"
            className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
          >
            Get started <ArrowRight className="size-4" />
          </Link>
          <a
            href={githubUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 rounded-lg border border-fd-border px-5 py-2.5 text-sm font-medium transition-colors hover:bg-fd-accent hover:text-fd-accent-foreground"
          >
            <GithubIcon className="size-4" /> View on GitHub
          </a>
        </div>
      </section>

      {/* 2. Journey — from prototype to shared link */}
      <section className="mx-auto w-full max-w-5xl px-6 pb-20">
        <div className="mb-12 text-center">
          <h2 className="text-2xl font-bold tracking-tight sm:text-3xl">
            From prototype to shared link
          </h2>
          <p className="mt-3 text-fd-muted-foreground">Three steps, no glue code.</p>
        </div>
        <div className="flex flex-col gap-12 sm:gap-16">
          {steps.map((step, i) => (
            <div
              key={step.n}
              className="grid items-center gap-6 sm:grid-cols-2 sm:gap-10"
            >
              {/* texte */}
              <div className={i % 2 === 1 ? 'sm:order-2' : ''}>
                <span className="inline-flex size-9 items-center justify-center rounded-full bg-fd-primary text-sm font-bold text-fd-primary-foreground">
                  {step.n}
                </span>
                <h3 className="mt-4 text-xl font-semibold">{step.title}</h3>
                <p className="mt-2 text-fd-muted-foreground">{step.body}</p>
              </div>
              {/* visuel */}
              <div
                className={`overflow-hidden rounded-xl border border-fd-border bg-fd-card shadow-sm ${
                  i % 2 === 1 ? 'sm:order-1' : ''
                }`}
              >
                {step.visual}
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* 3. Features */}
      <section className="border-t border-fd-border bg-fd-muted/30">
        <div className="mx-auto w-full max-w-5xl px-6 py-20">
          <div className="grid gap-px overflow-hidden rounded-xl border border-fd-border bg-fd-border sm:grid-cols-2 lg:grid-cols-3">
            {features.map((f) => (
              <div key={f.title} className="bg-fd-card p-6">
                <f.icon className="size-6 text-fd-primary" />
                <h3 className="mt-4 font-semibold">{f.title}</h3>
                <p className="mt-2 text-sm text-fd-muted-foreground">{f.body}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* 4. Architecture teaser */}
      <section className="mx-auto w-full max-w-5xl px-6 py-16 text-center">
        <h2 className="text-2xl font-bold tracking-tight">Built in layers</h2>
        <p className="mx-auto mt-4 max-w-2xl text-fd-muted-foreground">
          A small Rust core holds all the business logic, HTTP-agnostic. Thin inbound adapters
          (web, MCP, client serving) translate requests into core calls and own every auth
          decision. Outbound adapters speak to SQLite and a pluggable storage trait.
        </p>
        <Link
          href="/docs/how-it-works/architecture"
          className="mt-6 inline-flex items-center gap-1 text-sm font-medium text-fd-primary hover:underline"
        >
          Read the architecture <ArrowRight className="size-4" />
        </Link>
      </section>

      {/* 5. Final CTA */}
      <section className="border-t border-fd-border">
        <div className="mx-auto flex w-full max-w-5xl flex-col items-center gap-6 px-6 py-16 text-center">
          <h2 className="text-2xl font-bold tracking-tight sm:text-3xl">Get started in minutes</h2>
          <pre className="overflow-x-auto rounded-lg border border-fd-border bg-fd-card px-5 py-3 text-sm">
            <code>docker pull ghcr.io/owlnext-fr/latch</code>
          </pre>
          <Link
            href="/docs/quickstart"
            className="inline-flex items-center gap-2 rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground transition-opacity hover:opacity-90"
          >
            Read the quickstart <ArrowRight className="size-4" />
          </Link>
        </div>
      </section>

      {/* 6. Footer */}
      <footer className="border-t border-fd-border">
        <div className="mx-auto flex w-full max-w-5xl flex-col items-center justify-between gap-4 px-6 py-8 text-sm text-fd-muted-foreground sm:flex-row">
          <span className="inline-flex items-center gap-2">
            <LatchLogo className="size-4" /> latch
          </span>
          <nav className="flex flex-wrap items-center gap-4">
            <Link href="/docs" className="hover:text-fd-foreground">
              Docs
            </Link>
            <a href={githubUrl} target="_blank" rel="noopener noreferrer" className="hover:text-fd-foreground">
              GitHub
            </a>
            <a
              href={`${githubUrl}/blob/main/CHANGELOG.md`}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-fd-foreground"
            >
              Changelog
            </a>
            <span>MIT or Apache-2.0</span>
          </nav>
        </div>
      </footer>
    </main>
  );
}
