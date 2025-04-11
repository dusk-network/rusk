<script>
  import "./OverviewPanelDetailsItem.css";

  export let subtitle;
</script>

<div class="overview-panel__details-item">
  <div class="overview-panel__details-item-title"><slot /></div>
  <p class="overview-panel__details-item-subtitle">{subtitle}</p>
</div>
