<svelte:options immutable={true} />

<script>
  import { AppAnchor } from "$lib/components";
  import { Icon } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {DashboardNavItem[]} */
  export let items;
</script>

<nav {...$$restProps} class={makeClassName(["dashboard-nav", className])}>
  <ul class="dashboard-nav-list">
    {#each items as item (item.id)}
      {@const { icons, label, href } = item}
      <li>
        <AppAnchor {href} className="dashboard-nav-list__item">
          <span class="dashboard-nav-item-label">{label}</span>
          {#if icons && icons.length}
            <span class="dashboard-nav-item-icons">
              {#each icons as icon (icon.path)}
                <Icon path={icon.path} />
              {/each}
            </span>
          {/if}
        </AppAnchor>
      </li>
    {/each}
  </ul>
</nav>

<style lang="postcss">
  .dashboard-nav {
    background-color: var(--surface-color);
    border-radius: var(--control-border-radius-size);
    padding: 0.5rem 1.375rem;
    width: 100%;
  }

  .dashboard-nav-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    list-style: none;

    :global(&__item) {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      gap: 0.625rem;
    }

    .dashboard-nav-item-label {
      color: var(--on-surface-color);
      font-size: 1.125rem;
      font-style: normal;
      font-weight: 500;
      line-height: 1.6875rem;
    }

    .dashboard-nav-item-icons {
      color: var(--on-surface-color);
      display: flex;
      flex-direction: row;
      gap: 0.625rem;
    }
  }
</style>
