<script>
  import { AppImage } from "$lib/components";
  import { Card } from "$lib/dusk/components";

  /** @type {string} */
  const allSetIllustration = "/images/onboarding/all_set_illustration.svg";
</script>

<Card heading="You are all set!">
  <AppImage
    alt="All Set"
    className="all-set__illustration"
    height="182"
    src={allSetIllustration}
    width="205"
  />
</Card>

<style lang="postcss">
  :global(.all-set__illustration) {
    display: block;
    width: 50%;
    margin: 1em auto;
  }
</style>
