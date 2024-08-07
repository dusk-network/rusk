<svelte:options immutable={true} />

<script>
  import { UsageIndicator } from "$lib/components";
</script>

<section>
  <div style="width: 60%">
    <UsageIndicator value={45} />
  </div>
</section>
