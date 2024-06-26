<svelte:options immutable={true} />

<script>
  import { page } from "$app/stores";
  import { Card } from "$lib/dusk/components";
  const { error, status } = $page;
</script>

<section class="error-message">
  <Card className="error-message__card">
    <p class="error-message__card-content">
      Error {status}<br />
      {error?.message ?? ""}
    </p>
  </Card>
</section>

<style lang="postcss">
  .error-message {
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  :global(.error-message__card) {
    border: var(--control-border-size) solid var(--error-color);
    width: 100%;
  }

  :global(.error-message__card-content) {
    font-size: 1.75rem;
  }
</style>
