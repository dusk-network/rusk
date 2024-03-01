<script>
  import { Card } from "$lib/dusk/components";
  import { mdiHome } from "@mdi/js";
</script>

<section>
  <Card heading="My Card" />
  <Card iconPath={mdiHome} heading="My Card" />
  <Card iconPath={mdiHome} heading="My Card">
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
</section>

<style>
  p:first-of-type {
    margin-bottom: 1em;
  }
</style>
