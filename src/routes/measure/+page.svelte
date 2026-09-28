<script lang="ts">
  // Dev-only: вьюер модели без интерфейса — для замера силуэта через
  // window.__dbg (камера/сцена/рендерер). В приложении не используется.
  import MouseViewer from "$lib/MouseViewer.svelte";
</script>

<div class="stage">
  <MouseViewer />
</div>

<style>
  .stage {
    position: relative;
    width: 100vw;
    height: 100vh;
    background: #101014;
  }
</style>
