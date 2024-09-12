<script>
  import { Card } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { mdiHome } from "@mdi/js";
  import ToggleableCard from "$lib/containers/Cards/ToggleableCard.svelte";
</script>

<section>
  <Card>
    <header slot="header">
      <h3>Card Heading</h3>
    </header>
    <p>
      Lorem ipsum dolor sit amet, consectetur adipiscing elit. In viverra
      tincidunt dolor, nec imperdiet felis tristique non. Nunc quis tempus est,
      sit amet euismod arcu. Sed malesuada scelerisque metus, ut tempor metus.
    </p>
    <p>
      Nam at nisi nibh. Nulla augue ex, imperdiet mattis commodo vel,
      ullamcorper in velit. Aenean eros dolor, condimentum sed tempor et, ornare
      in erat.
    </p>
    <footer slot="footer">
      <small>This is an example footer.</small>
    </footer>
  </Card>
  <hr />
  <Card>
    <p>
      Lorem ipsum dolor sit amet, consectetur adipiscing elit. In viverra
      tincidunt dolor, nec imperdiet felis tristique non. Nunc quis tempus est,
      sit amet euismod arcu. Sed malesuada scelerisque metus, ut tempor metus.
    </p>
    <p>
      Nam at nisi nibh. Nulla augue ex, imperdiet mattis commodo vel,
      ullamcorper in velit. Aenean eros dolor, condimentum sed tempor et, ornare
      in erat.
    </p>
  </Card>
  <hr />
  <IconHeadingCard heading="No place like home" iconPath={mdiHome} reverse>
    <p>
      Lorem ipsum dolor sit amet, consectetur adipiscing elit. In viverra
      tincidunt dolor, nec imperdiet felis tristique non. Nunc quis tempus est,
      sit amet euismod arcu. Sed malesuada scelerisque metus, ut tempor metus.
    </p>
    <p>
      Nam at nisi nibh. Nulla augue ex, imperdiet mattis commodo vel,
      ullamcorper in velit. Aenean eros dolor, condimentum sed tempor et, ornare
      in erat.
    </p>
  </IconHeadingCard>
  <hr />
  <IconHeadingCard heading="No place like home" iconPath={mdiHome}>
    <p>
      Lorem ipsum dolor sit amet, consectetur adipiscing elit. In viverra
      tincidunt dolor, nec imperdiet felis tristique non. Nunc quis tempus est,
      sit amet euismod arcu. Sed malesuada scelerisque metus, ut tempor metus.
    </p>
    <p>
      Nam at nisi nibh. Nulla augue ex, imperdiet mattis commodo vel,
      ullamcorper in velit. Aenean eros dolor, condimentum sed tempor et, ornare
      in erat.
    </p>
  </IconHeadingCard>
  <hr />
  <ToggleableCard heading="No place like home" iconPath={mdiHome}>
    <p>
      Lorem ipsum dolor sit amet, consectetur adipiscing elit. In viverra
      tincidunt dolor, nec imperdiet felis tristique non. Nunc quis tempus est,
      sit amet euismod arcu. Sed malesuada scelerisque metus, ut tempor metus.
    </p>
    <p>
      Nam at nisi nibh. Nulla augue ex, imperdiet mattis commodo vel,
      ullamcorper in velit. Aenean eros dolor, condimentum sed tempor et, ornare
      in erat.
    </p>
  </ToggleableCard>
</section>

<style lang="postcss">
  p:first-of-type {
    margin-bottom: 1em;
  }
</style>
