<script>
  import { settingsStore } from "$lib/stores";
  import { AppImage } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";

  const { darkMode } = $settingsStore;

  const allSetIllustration = "/images/onboarding/all_set_illustration.svg";
  const allSetIllustrationLight =
    "/images/onboarding/all_set_illustration_light.svg";
</script>

<IconHeadingCard gap="medium" heading="You are all set!">
  <AppImage
    alt="All Set"
    className="all-set__illustration"
    height="182"
    src={darkMode ? allSetIllustrationLight : allSetIllustration}
    width="205"
  />
</IconHeadingCard>

<style lang="postcss">
  :global(.all-set__illustration) {
    display: block;
    width: 50%;
    margin: 0 auto 1em auto;
  }
</style>
