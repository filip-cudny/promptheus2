<script lang="ts">
  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";
</script>

<main>
  <p>Prompt Dialog: {promptName} ({promptId})</p>
</main>
