<script>
  import { Banner } from "$lib/components";
  import { Anchor } from "$lib/dusk/components";
</script>

<section>
  <Banner title="Example Info" variant="info">
    <p>
      For more information, go on our <Anchor href="dusk.network"
        >website</Anchor
      >.
    </p>
  </Banner>
  <Banner title="Example Warning" variant="warning">
    <p>This transaction will be public.</p>
  </Banner>
  <Banner title="Example Error" variant="error">
    <p>Invalid address entered.</p>
  </Banner>
  <h3>Long Content</h3>
  <Banner
    title="Setting a password for your web wallet is optional."
    variant="info"
  >
    <p>
      Doing so allows you the convenience of opening your wallet file using a
      password, but it weakens the overall security. Not using a password
      requires you to input the full mnemonic to open your wallet.
    </p>
  </Banner>
</section>
