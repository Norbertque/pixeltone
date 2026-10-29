<script lang="ts">
  import type { GuestPermissions } from '../api';
  import { jam } from '../jam.svelte';
  import { qrShape } from '../qr';
  import { toasts } from '../toasts.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Switch from './Switch.svelte';

  const MAX_GUESTS = 32;
  const PERMISSIONS: { key: keyof GuestPermissions; title: string; body: string }[] = [
    { key: 'addTracks', title: 'Add tracks', body: 'From this PC’s library, or from their phones' },
    { key: 'changeOwnTracks', title: 'Change their tracks', body: 'Remove and move the tracks they added' },
    { key: 'controlPlayback', title: 'Control playback', body: 'Play, pause, skip and seek' },
  ];

  let dialog: HTMLDialogElement | undefined = $state();
  const view = $derived(jam.view);
  const qr = $derived(view ? qrShape(view.link) : null);
  /** The PC's name, as guests see it ("Join BIG-ONE's jam"). */
  const pcName = $derived(view ? new URLSearchParams(view.link.split('#')[1] ?? '').get('n') : null);

  $effect(() => {
    if (!dialog) return;
    if (jam.dialogOpen && !dialog.open) dialog.showModal();
    else if (!jam.dialogOpen && dialog.open) dialog.close();
  });

  function setPermission(key: keyof GuestPermissions, on: boolean): void {
    if (view) void jam.setPermissions({ ...view.permissions, [key]: on });
  }

  async function copyLink(): Promise<void> {
    if (!view) return;
    try {
      await navigator.clipboard.writeText(view.link);
      toasts.show('Link copied');
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }
</script>

<dialog
  bind:this={dialog}
  class={['dialog', { running: view }]}
  aria-labelledby="jam-title"
  onclose={() => (jam.dialogOpen = false)}
>
  {#if view && qr}
    <div class="layout">
      <section class="invite" aria-label="Invite">
        <h2 id="jam-title" class="headline-small"><Icon name="groups" /> Jam</h2>
        <div class="qr">
          <svg viewBox="-2 -2 {qr.size + 4} {qr.size + 4}" role="img" aria-label="QR code to join the jam">
            <rect x="-2" y="-2" width={qr.size + 4} height={qr.size + 4} fill="#fff" />
            <path d={qr.path} fill="#000" shape-rendering="crispEdges" />
          </svg>
        </div>
        <p class="body-medium hint">
          Friends on the same Wi-Fi scan this with their phone to join{pcName ? ` ${pcName}’s jam` : ''}. They need
          Pixeltone for Android; the page it opens says where to get it.
        </p>
        <Button variant="text" icon="link" onclick={() => void copyLink()}>Copy link</Button>
      </section>

      <section class="manage">
        {#if view.requests.length}
          <div class="group">
            <h3 class="title-medium">Rejoin requests</h3>
            <ul class="list">
              {#each view.requests as r (r.id)}
                <li class="request">
                  <Icon name="person_add" />
                  <span class="body-large name">{r.name} wants to rejoin the jam</span>
                  <Button variant="text" disabled={jam.busy} onclick={() => void jam.answer(r.id, false)}>Decline</Button>
                  <Button variant="tonal" disabled={jam.busy} onclick={() => void jam.answer(r.id, true)}>Allow</Button>
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        <div class="group">
          <h3 class="title-medium">Guests <span class="count body-medium">{view.guests.length} of {MAX_GUESTS}</span></h3>
          {#if view.guests.length}
            <ul class="list">
              {#each view.guests as g (g.id)}
                <li class="guest">
                  <Icon name="person" />
                  <span class="body-large name">{g.name}</span>
                  <IconButton
                    icon="person_remove"
                    label="Remove {g.name} from the jam"
                    size="xs"
                    disabled={jam.busy}
                    onclick={() => void jam.remove(g)}
                  />
                </li>
              {/each}
            </ul>
          {:else}
            <p class="body-medium muted">No one yet. Guests show up here as they join.</p>
          {/if}
        </div>

        <div class="group">
          <h3 class="title-medium">Guest permissions</h3>
          {#each PERMISSIONS as p (p.key)}
            <div class="permission">
              <span class="text">
                <span class="body-large">{p.title}</span>
                <span class="body-small muted">{p.body}</span>
              </span>
              <Switch
                label={p.title}
                checked={view.permissions[p.key]}
                disabled={jam.busy}
                onchange={(on) => setPermission(p.key, on)}
              />
            </div>
          {/each}
          <p class="body-small muted">The volume stays yours.</p>
        </div>

        <div class="actions">
          <Button variant="text" icon="stop_circle" disabled={jam.busy} onclick={() => void jam.end()}>End jam</Button>
          <Button variant="filled" onclick={() => (jam.dialogOpen = false)}>Done</Button>
        </div>
      </section>
    </div>
  {:else}
    <div class="intro">
      <h2 id="jam-title" class="headline-small"><Icon name="groups" /> Start a jam</h2>
      <p class="body-medium">
        Friends on the same Wi-Fi scan a QR code and add tracks to this queue from their phones. You decide what else
        they may do, and you can remove anyone.
      </p>
      <p class="body-medium muted">
        The jam goes on until you end it or close Pixeltone. Tracks guests send from their phones are copied to this PC
        and deleted when it ends; nothing played in a jam counts in your listening history.
      </p>
      <div class="actions">
        <Button variant="text" onclick={() => (jam.dialogOpen = false)}>Cancel</Button>
        <Button variant="filled" icon="groups" disabled={jam.busy} onclick={() => void jam.start()}>Start jam</Button>
      </div>
    </div>
  {/if}
</dialog>

<style>
  .dialog {
    width: min(480px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 24px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }
  .dialog.running {
    width: min(860px, calc(100vw - 32px));
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px;
  }
  h2 :global(.icon) {
    color: var(--md-sys-color-primary);
  }
  h3 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0 0 8px;
  }
  p {
    margin: 0 0 12px;
  }
  .muted,
  .count {
    color: var(--md-sys-color-on-surface-variant);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 340px) minmax(0, 1fr);
    gap: 32px;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .invite {
    display: grid;
    align-content: start;
    justify-items: center;
    text-align: center;
  }
  .invite h2 {
    justify-self: start;
  }
  .qr {
    width: 100%;
    max-width: 300px;
    margin-bottom: 16px;
    padding: 12px;
    border-radius: var(--shape-lg);
    background: #fff;
  }
  .qr svg {
    display: block;
    width: 100%;
    height: auto;
  }
  .hint {
    color: var(--md-sys-color-on-surface-variant);
  }

  .manage {
    display: grid;
    align-content: start;
    gap: 20px;
    min-width: 0;
  }
  .list {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .guest,
  .request {
    display: grid;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .guest {
    grid-template-columns: auto 1fr auto;
  }
  .request {
    grid-template-columns: auto 1fr auto auto;
    padding: 4px 4px 4px 12px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }
  .name {
    overflow: hidden;
    color: var(--md-sys-color-on-surface);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .request .name {
    color: inherit;
    white-space: normal;
  }
  .request :global(.button.text) {
    color: inherit;
  }
  .permission {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 6px 0;
  }
  .permission .text {
    display: grid;
    flex: 1;
  }
  .group > .body-small {
    margin: 4px 0 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .intro .actions {
    margin-top: 8px;
  }
</style>
