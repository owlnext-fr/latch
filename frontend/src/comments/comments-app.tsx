import { useMemo, useReducer, useState } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { SameOriginPicker } from './picker/same-origin-picker'
import type { FrameRef } from './picker/picker'
import type { AnchorDescriptor } from './anchor/descriptor'
import { parseAnchor, serializeAnchor } from './anchor/descriptor'
import type { ShellRect } from './picker/picker'
import { useFollow } from './follow/use-follow'
import type { PinInput } from './follow/controller'
import {
  useCommentList,
  useCreatePin,
  useAddReply,
  useEditMessage,
  useDeleteMessage,
  useDeletePin,
} from './data/use-comments'
import type { CommentsAdapter, CommentPin } from './data/adapter'
import { initialPickState, pickReducer } from './state/pick-machine'
import { OverlayLayer } from './ui/overlay-layer'
import { firstLetter } from './ui/pin-label'
import { ComposePopup } from './ui/compose-popup'
import { ThreadPopup } from './ui/thread-popup'
import { ActionBar } from './ui/action-bar'
import { CommentsDrawer } from './ui/comments-drawer'
import { getStoredName } from './ui/name-prompt'

interface CommentsAppProps {
  cacheKey: string
  frame: FrameRef
  adapter: CommentsAdapter
}

/** Dernier auteur du fil (fallback pour le nom de réponse). */
function lastAuthor(pin: CommentPin): string {
  return pin.messages.at(-1)?.author_name ?? ''
}

/** Composant interne : suppose le QueryClientProvider déjà monté. */
function CommentsInner({ cacheKey, frame, adapter }: Readonly<CommentsAppProps>) {
  const picker = useMemo(() => new SameOriginPicker(frame), [frame])

  const list = useCommentList(cacheKey, adapter)
  const createPin = useCreatePin(cacheKey, adapter)
  const addReply = useAddReply(cacheKey, adapter)
  const editMessage = useEditMessage(cacheKey, adapter)
  const deleteMessage = useDeleteMessage(cacheKey, adapter)
  const deletePin = useDeletePin(cacheKey, adapter)

  const pins = useMemo(() => list.data?.pins ?? [], [list.data])
  const pinInputs: PinInput[] = useMemo(
    () =>
      pins
        .map((p) => {
          const anchor = parseAnchor(p.anchor)
          return anchor ? { id: p.id, anchor } : null
        })
        .filter((x): x is PinInput => x !== null),
    [pins],
  )

  const positions = useFollow(picker, pinInputs)
  const [pick, dispatch] = useReducer(pickReducer, initialPickState)
  const [pinsVisible, setPinsVisible] = useState(true)
  const [activePinId, setActivePinId] = useState<number | null>(null)
  const [drawerOpen, setDrawerOpen] = useState(false)

  const activePin = pins.find((p) => p.id === activePinId) ?? null
  const activePosition = positions.find((p) => p.id === activePinId) ?? null

  function onPick(anchor: AnchorDescriptor, rect: ShellRect) {
    dispatch({ type: 'CAPTURE', anchor, rect })
  }

  function submitNewComment(v: { author_name: string; body: string }) {
    if (pick.mode !== 'compose') return
    createPin.mutate(
      { anchor: serializeAnchor(pick.anchor), author_name: v.author_name, body: v.body },
      { onSuccess: () => dispatch({ type: 'SUBMITTED' }) },
    )
  }

  function focusPinFromList(pinId: number) {
    const pin = pins.find((p) => p.id === pinId)
    const anchor = pin ? parseAnchor(pin.anchor) : null
    const el = anchor ? picker.resolve(anchor).element : null
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' })
    setActivePinId(pinId)
    setDrawerOpen(false)
  }

  return (
    <>
      <OverlayLayer
        picker={picker}
        positions={pinsVisible ? positions : []}
        pickMode={pick.mode === 'pick'}
        onPick={onPick}
        onPinClick={setActivePinId}
        activePinId={activePinId}
        labelOf={(id) => firstLetter(pins.find((p) => p.id === id)?.messages[0]?.author_name ?? '')}
      />
      {pick.mode === 'compose' && (
        <ComposePopup
          rect={pick.rect}
          submitting={createPin.isPending}
          onSubmit={submitNewComment}
          onCancel={() => dispatch({ type: 'CANCEL' })}
        />
      )}
      {activePin !== null && activePosition !== null && (
        <ThreadPopup
          pin={activePin}
          position={activePosition}
          capabilities={adapter.capabilities}
          busy={addReply.isPending || editMessage.isPending || deleteMessage.isPending}
          onReply={(body) =>
            addReply.mutate({
              pinId: activePin.id,
              author_name: getStoredName() || lastAuthor(activePin),
              body,
            })
          }
          onEdit={(messageId, body) => editMessage.mutate({ messageId, body })}
          onDelete={(messageId) => deleteMessage.mutate(messageId)}
          onDeletePin={() => {
            deletePin.mutate(activePin.id)
            setActivePinId(null)
          }}
          onClose={() => setActivePinId(null)}
        />
      )}
      <CommentsDrawer
        open={drawerOpen}
        pins={pins}
        statusOf={(id) => positions.find((p) => p.id === id)?.status}
        onClose={() => setDrawerOpen(false)}
        onSelect={focusPinFromList}
      />
      <ActionBar
        capabilities={adapter.capabilities}
        pinCount={pins.length}
        pickActive={pick.mode === 'pick'}
        pinsVisible={pinsVisible}
        onTogglePick={() =>
          dispatch(pick.mode === 'pick' ? { type: 'CANCEL' } : { type: 'ENTER_PICK' })
        }
        onToggleVisible={() => setPinsVisible((v) => !v)}
        onOpenList={() => setDrawerOpen((o) => !o)}
      />
    </>
  )
}

export function CommentsApp(props: Readonly<CommentsAppProps>) {
  const client = useMemo(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    [],
  )
  return (
    <QueryClientProvider client={client}>
      <CommentsInner {...props} />
    </QueryClientProvider>
  )
}
