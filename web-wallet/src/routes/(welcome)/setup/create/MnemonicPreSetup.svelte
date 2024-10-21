<script>
  import { Agreement } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { Banner } from "$lib/components";

  /** @type {boolean} */
  export let isValid = false;

  /** @type {boolean} */
  let agreementOneChecked = isValid || false;

  /** @type {boolean} */
  let agreementTwoChecked = isValid || false;

  $: isValid = agreementOneChecked && agreementTwoChecked;
</script>

<IconHeadingCard gap="medium" heading="Securely store your seed phrase!">
  <Agreement
    name="mnemonic_agreement_one"
    bind:checked={agreementOneChecked}
    label="I understand that without my seed phrase I cannot access my wallet and its assets."
  />
  <Agreement
    name="mnemonic_agreement_two"
    bind:checked={agreementTwoChecked}
    label="I understand that securing my recovery phrase is my own responsibility.
        Only I have access to my recovery phrase"
  />
</IconHeadingCard>

<Banner
  title="Dusk will not save and cannot retrieve your passphrase."
  variant="warning"
>
  <p>To proceed, please check all the relevant boxes.</p>
</Banner>
