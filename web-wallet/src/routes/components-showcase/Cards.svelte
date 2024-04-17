<script>
  import { Card, Icon } from "$lib/dusk/components";
  import { mdiHome } from "@mdi/js";
</script>

<section>
  <Card>
    <header slot="header" class="card__header">
      <Icon path={mdiHome} />
      <h3>Card Title</h3>
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
</section>

<style lang="postcss">
  p:first-of-type {
    margin-bottom: 1em;
  }

  :global(.card__header) {
    display: flex;
    align-items: center;
    gap: var(--small-gap);
  }
</style>
