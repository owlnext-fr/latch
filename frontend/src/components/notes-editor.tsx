import { useState } from 'react'
import type { ComponentType } from 'react'
import { useTranslation } from 'react-i18next'
import {
  useEditor,
  EditorContent,
  useEditorState,
  type Editor,
} from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from 'tiptap-markdown'
import {
  Bold,
  Italic,
  Heading1,
  Heading2,
  List,
  ListOrdered,
  Quote,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { MarkdownView } from '@/lib/markdown'

declare module '@tiptap/core' {
  interface Storage {
    markdown: { getMarkdown: () => string }
  }
}

/** Bouton de la barre d'outils : icône + état actif (aria-pressed). */
function ToolButton({
  active,
  onClick,
  icon: Icon,
  label,
}: Readonly<{
  active: boolean
  onClick: () => void
  icon: ComponentType<{ className?: string }>
  label: string
}>) {
  return (
    <Button
      type="button"
      size="sm"
      variant="ghost"
      aria-label={label}
      aria-pressed={active}
      title={label}
      className={active ? 'bg-accent text-accent-foreground' : ''}
      onClick={onClick}
    >
      <Icon className="size-4" />
    </Button>
  )
}

/** Barre d'outils réactive (états actifs dérivés de l'éditeur). */
function Toolbar({ editor }: Readonly<{ editor: Editor }>) {
  const { t } = useTranslation()
  const s = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive('bold'),
      italic: editor.isActive('italic'),
      h1: editor.isActive('heading', { level: 1 }),
      h2: editor.isActive('heading', { level: 2 }),
      bullet: editor.isActive('bulletList'),
      ordered: editor.isActive('orderedList'),
      quote: editor.isActive('blockquote'),
    }),
  })

  return (
    <div className="flex flex-wrap gap-1 rounded-md border border-input p-1">
      <ToolButton
        active={s.bold}
        onClick={() => editor.chain().focus().toggleBold().run()}
        icon={Bold}
        label={t('deploy.notes_bold')}
      />
      <ToolButton
        active={s.italic}
        onClick={() => editor.chain().focus().toggleItalic().run()}
        icon={Italic}
        label={t('deploy.notes_italic')}
      />
      <ToolButton
        active={s.h1}
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        icon={Heading1}
        label={t('deploy.notes_h1')}
      />
      <ToolButton
        active={s.h2}
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        icon={Heading2}
        label={t('deploy.notes_h2')}
      />
      <ToolButton
        active={s.bullet}
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        icon={List}
        label={t('deploy.notes_bullet')}
      />
      <ToolButton
        active={s.ordered}
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        icon={ListOrdered}
        label={t('deploy.notes_ordered')}
      />
      <ToolButton
        active={s.quote}
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        icon={Quote}
        label={t('deploy.notes_quote')}
      />
    </div>
  )
}

/**
 * Éditeur WYSIWYG restreint au périmètre markdown partagé (titres, gras,
 * italique, listes, citation). Sérialise en markdown via tiptap-markdown.
 * Barre d'outils pour formater par boutons (l'édition est WYSIWYG, pas en
 * markdown brut). Onglet Aperçu = rendu réel (MarkdownView), identique à
 * l'overlay client.
 */
export function NotesEditor({
  value,
  onChange,
}: Readonly<{ value: string; onChange: (md: string) => void }>) {
  const { t } = useTranslation()
  const [tab, setTab] = useState<'write' | 'preview'>('write')

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        // Hors périmètre → désactivés.
        code: false,
        codeBlock: false,
        strike: false,
        horizontalRule: false,
      }),
      Markdown,
    ],
    content: value,
    // borne back = 10000 (MAX_RELEASE_NOTES_LEN) ; éditeur non-natif (tiptap), pas de maxLength
    onUpdate: ({ editor }) => {
      onChange(editor.storage.markdown.getMarkdown())
    },
  })

  return (
    <div className="flex flex-col gap-2" data-testid="notes-editor">
      <div className="flex gap-1">
        <Button
          type="button"
          size="sm"
          variant={tab === 'write' ? 'secondary' : 'ghost'}
          onClick={() => setTab('write')}
        >
          {t('deploy.notes_write')}
        </Button>
        <Button
          type="button"
          size="sm"
          variant={tab === 'preview' ? 'secondary' : 'ghost'}
          onClick={() => setTab('preview')}
          data-testid="notes-preview-tab"
        >
          {t('deploy.notes_preview')}
        </Button>
      </div>

      {tab === 'write' ? (
        <div className="flex flex-col gap-2">
          {editor && <Toolbar editor={editor} />}
          <EditorContent
            editor={editor}
            className="rounded-md border border-input px-3 py-2 text-sm [&_.ProseMirror]:min-h-[120px] [&_.ProseMirror]:space-y-2 [&_.ProseMirror]:outline-none [&_.ProseMirror_blockquote]:text-muted-foreground [&_.ProseMirror_blockquote]:border-l-2 [&_.ProseMirror_blockquote]:pl-3 [&_.ProseMirror_blockquote]:italic [&_.ProseMirror_h1]:text-lg [&_.ProseMirror_h1]:font-semibold [&_.ProseMirror_h2]:text-base [&_.ProseMirror_h2]:font-semibold [&_.ProseMirror_h3]:text-sm [&_.ProseMirror_h3]:font-semibold [&_.ProseMirror_ol]:list-decimal [&_.ProseMirror_ol]:pl-5 [&_.ProseMirror_ul]:list-disc [&_.ProseMirror_ul]:pl-5"
          />
        </div>
      ) : (
        <div className="rounded-md border border-input px-3 py-2">
          <MarkdownView source={value} />
        </div>
      )}
    </div>
  )
}
