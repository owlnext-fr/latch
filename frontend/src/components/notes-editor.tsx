import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from 'tiptap-markdown'
import { Button } from '@/components/ui/button'
import { MarkdownView } from '@/lib/markdown'

declare module '@tiptap/core' {
  interface Storage {
    markdown: { getMarkdown: () => string }
  }
}

/**
 * Éditeur WYSIWYG restreint au périmètre markdown partagé (titres, gras,
 * italique, listes, citation). Sérialise en markdown via tiptap-markdown.
 * Onglet Aperçu = rendu réel (MarkdownView), identique à l'overlay client.
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
        <EditorContent
          editor={editor}
          className="prose prose-sm max-w-none rounded-md border border-input px-3 py-2 [&_.ProseMirror]:min-h-[120px] [&_.ProseMirror]:outline-none"
        />
      ) : (
        <div className="prose prose-sm max-w-none rounded-md border border-input px-3 py-2">
          <MarkdownView source={value} />
        </div>
      )}
    </div>
  )
}
