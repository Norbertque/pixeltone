<script lang="ts">
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import FolderView from './FolderView.svelte';

  interface Props {
    path: string | null;
  }
  let { path }: Props = $props();

  const name = $derived(path ? (path.split(/[\\/]/).filter(Boolean).pop() ?? path) : 'Folders');
</script>

<ScreenHeader title={name} />
<div class="body">
  <FolderView {path} />
</div>

<style>
  .body {
    padding-bottom: 32px;
  }
</style>
