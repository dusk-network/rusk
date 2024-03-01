<script>
  import { Card } from "$lib/dusk/components";

  /** @type {string} */
  const allSetIllustration = "/images/onboarding/all_set_illustration.svg";
</script>

<Card heading="You are all set!">
  <img
    src={allSetIllustration}
    width="205"
    height="182"
    id="all_set_illustration"
    alt="All Set"
  />
</Card>

<style>
  #all_set_illustration {
    display: block;
    width: 50%;
    margin: 1em auto;
  }
</style>
