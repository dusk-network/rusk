<svelte:options immutable={true} />

<script>
  import { MigrateContract } from "$lib/containers";
</script>

<section>
  <MigrateContract migrationNetwork="Testnet" />
</section>
