<svelte:options immutable={true} />

<script>
  import { walletStore } from "$lib/stores";
  import { Receive } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { mdiArrowTopRight } from "@mdi/js";

  $: ({ currentAddress } = $walletStore);
</script>

<IconHeadingCard
  gap="medium"
  heading="Transfer"
  iconPath={mdiArrowTopRight}
  reverse
>
  <Receive address={currentAddress} />
</IconHeadingCard>
