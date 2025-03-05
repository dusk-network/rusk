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
    title="Setting a password for your web wallet is recommended."
    variant="info"
  >
    <p>
      Without a password, you will need to input your full mnemonic each time to
      access your wallet. This could expose your mnemonic to potential threats.
      Setting a password is highly recommended for better wallet security.
    </p>
  </Banner>
</section>
