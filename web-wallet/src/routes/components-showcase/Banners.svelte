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
</section>
