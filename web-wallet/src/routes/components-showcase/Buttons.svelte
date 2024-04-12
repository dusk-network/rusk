<script>
  import { mdiHome } from "@mdi/js";

  import { Button } from "$lib/dusk/components";
  import AnchorButton from "$lib/dusk/components/AnchorButton/AnchorButton.svelte";
</script>

<section>
  <Button icon={{ path: mdiHome }} text="Primary" />
  <Button disabled={true} icon={{ path: mdiHome }} text="Primary - Disabled" />
  <br />
  <Button icon={{ path: mdiHome }} text="Secondary" variant="secondary" />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    text="Secondary - Disabled"
    variant="secondary"
  />
  <br />
  <Button icon={{ path: mdiHome }} text="Tertiary" variant="tertiary" />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    text="Tertiary - Disabled"
    variant="tertiary"
  />
  <br />
</section>
<br />
<section>
  <AnchorButton href="/" icon={{ path: mdiHome }} text="Anchor - Primary" />
  <AnchorButton
    href="/"
    disabled={true}
    icon={{ path: mdiHome }}
    text="Anchor - Primary - Disabled"
  />
  <AnchorButton
    href="/"
    icon={{ path: mdiHome }}
    text="Anchor - Secondary"
    variant="secondary"
  />
  <AnchorButton
    href="/"
    disabled={true}
    icon={{ path: mdiHome }}
    text="Anchor - Secondary - Disabled"
    variant="secondary"
  />
  <AnchorButton
    href="/"
    icon={{ path: mdiHome }}
    text="Anchor - Tertiary"
    variant="tertiary"
  />
  <AnchorButton
    href="/"
    disabled={true}
    icon={{ path: mdiHome }}
    text="Tertiary - Disabled"
    variant="tertiary"
  />
  <br />
</section>
<br />
<section>
  <Button icon={{ path: mdiHome }} size="small" text="Primary – Small" />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    size="small"
    text="Primary - Small - Disabled"
  />
  <Button
    icon={{ path: mdiHome }}
    text="Secondary – Small"
    size="small"
    variant="secondary"
  />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    size="small"
    text="Secondary - Small - Disabled"
    variant="secondary"
  />
  <Button
    icon={{ path: mdiHome }}
    size="small"
    text="Tertiary - Small"
    variant="tertiary"
  />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    size="small"
    text="Tertiary - Disabled"
    variant="tertiary"
  />
</section>
<br />
<section>
  <Button icon={{ path: mdiHome }} />
  <Button disabled={true} icon={{ path: mdiHome }} />
  <Button icon={{ path: mdiHome }} variant="secondary" />
  <Button disabled={true} icon={{ path: mdiHome }} variant="secondary" />
  <Button icon={{ path: mdiHome }} variant="tertiary" />
  <Button disabled={true} icon={{ path: mdiHome }} variant="tertiary" />
</section>
<section>
  <Button icon={{ path: mdiHome }} size="small" />
  <Button disabled={true} icon={{ path: mdiHome }} size="small" />
  <Button icon={{ path: mdiHome }} size="small" variant="secondary" />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    size="small"
    variant="secondary"
  />
  <Button icon={{ path: mdiHome }} size="small" variant="tertiary" />
  <Button
    disabled={true}
    icon={{ path: mdiHome }}
    size="small"
    variant="tertiary"
  />
</section>

<style lang="postcss">
  section {
    align-items: flex-start;
    justify-content: unset !important;
    margin: 1rem 0 !important;
  }
</style>
