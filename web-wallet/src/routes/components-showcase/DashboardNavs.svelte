<svelte:options immutable={true} />

<script>
  import { mdiPlusBoxOutline, mdiSwapHorizontal, mdiTimerSand } from "@mdi/js";
  import { DashboardNav } from "$lib/components";

  const items = [
    {
      href: "#",
      icons: [{ path: mdiSwapHorizontal }],
      id: "item-1",
      label: "Something",
    },
    {
      href: "#",
      icons: [{ path: mdiPlusBoxOutline }],
      id: "item-2",
      label: "Send",
    },
    {
      href: "#",
      icons: [{ path: mdiTimerSand }],
      id: "item-3",
      label: "Receive",
    },
    {
      href: "#",
      icons: [
        { path: mdiTimerSand },
        { path: mdiSwapHorizontal },
        { path: mdiPlusBoxOutline },
      ],
      id: "item-4",
      label: "Stake",
    },
  ];
</script>

<section>
  <DashboardNav {items} />
</section>
