<script>
  import { mdiKeyOutline } from "@mdi/js";
  import { Textbox } from "$lib/dusk/components";
  import { ToggleableCard } from "$lib/containers/Cards";
  import { Banner } from "$lib/components";

  /** @type {string} */
  export let password = "";

  /** @type {boolean} */
  export let isValid = false;

  /** @type {boolean} */
  export let isToggled = true;

  /** @type {string} */
  let confirmPassword = "";

  $: isValid =
    !isToggled ||
    (password.length >= 8 &&
      confirmPassword.length >= 8 &&
      password === confirmPassword);

  $: if (isToggled) {
    password = "";
    confirmPassword = "";
  }
</script>

<ToggleableCard heading="Password" iconPath={mdiKeyOutline} bind:isToggled>
  <p>Please store your password safely.</p>
  <Textbox
    className="password-input"
    type="password"
    autocomplete="new-password"
    bind:value={password}
    placeholder="Set Password"
  />
  <div class="confirm-password">
    <Textbox
      className="password-input"
      type="password"
      autocomplete="new-password"
      bind:value={confirmPassword}
      placeholder="Confirm Password"
    />
    {#if password.length < 8}
      <span class="confirm-password--meta"
        >Password must be at least 8 characters</span
      >
    {:else if confirmPassword && password !== confirmPassword}
      <span
        class="confirm-password--meta
						confirm-password--meta--error">Passwords do not match</span
      >
    {/if}
  </div>
</ToggleableCard>

<Banner
  title="Setting a password for your web wallet is recommended."
  variant="info"
>
  <p>
    Without a password, you will need to input your full mnemonic each time to
    access your wallet. This could expose your mnemonic to potential threats.
    Setting a password is highly recommended for better wallet security.
  </p>
</Banner>

<style lang="postcss">
  .confirm-password {
    &--meta {
      display: inline-block;
      font-size: 0.75em;
      margin-top: 0.8em;
      margin-left: 1em;
      opacity: 0.5;

      &--error {
        color: var(--error-color);
        opacity: 1;
      }
    }
  }

  :global(.password-input) {
    display: flex;
    margin-top: 1em;
    width: 100%;
  }
</style>
