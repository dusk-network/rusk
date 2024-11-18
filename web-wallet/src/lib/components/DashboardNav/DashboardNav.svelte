<svelte:options immutable={true} />

<script>
  import { Card, Icon } from "$lib/dusk/components";
  import { AppAnchor } from "$lib/components";

  /** @type {DashboardNavItem[]} */
  export let items;
</script>

<Card>
  <nav class="dashboard-nav" aria-label="Transaction and operations navigation">
    {#each items as item (item.id)}
      {@const { icons, label, href } = item}
      <AppAnchor
        {href}
        tabindex="0"
        className="dashboard-nav__item"
        role="menuitem"
      >
        <span class="dashboard-nav__item-label">{label}</span>
        {#if icons?.length}
          <span class="dashboard-nav__item-icons">
            {#each icons as icon (icon.path)}
              <Icon path={icon.path} />
            {/each}
          </span>
        {/if}
      </AppAnchor>
    {/each}
  </nav>
</Card>

<style lang="postcss">
  .dashboard-nav {
    :global(&__item) {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      padding: 0.5rem 0;
      color: var(--on-surface-color) !important;

      &:first-child {
        padding-top: 0;
      }

      &:last-child {
        padding-bottom: 0;
      }

      &:hover {
        color: var(--anchor-color-on-surface) !important;
        text-decoration: none !important;
      }
    }

    &__item-label {
      font-size: 1.125rem;
      font-style: normal;
      font-weight: 500;
      line-height: 1.6875rem;
    }

    &__item-icons {
      display: flex;
      flex-direction: row;
      gap: 0.625rem;
    }
  }
</style>
