<script>
  import { mdiHome } from "@mdi/js";

  import { Button } from "$lib/dusk/components";
</script>

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
