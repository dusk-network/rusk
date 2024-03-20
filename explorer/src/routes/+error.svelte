<svelte:options immutable={true} />

<script>
  import { page } from "$app/stores";

  const { error, status } = $page;
</script>

<section class="error-message">
  <h2>Error - {status}</h2>
  <p>{error?.message ?? ""}</p>
</section>

<style lang="postcss">
  .error-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }
</style>
