<script>
  import { settingsStore } from "$lib/stores";
  import { AppImage } from "$lib/components";
  import { Card } from "$lib/dusk/components";

  const { darkMode } = $settingsStore;

  const allSetIllustration = "/images/onboarding/all_set_illustration.svg";
  const allSetIllustrationLight =
    "/images/onboarding/all_set_illustration_light.svg";
</script>

<Card heading="You are all set!">
  <AppImage
    alt="All Set"
    className="all-set__illustration"
    height="182"
    src={darkMode ? allSetIllustrationLight : allSetIllustration}
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
