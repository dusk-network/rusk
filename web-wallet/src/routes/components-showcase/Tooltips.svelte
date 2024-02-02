<script>
	import { Tooltip } from "$lib/dusk/components";
</script>

<section>
	<span
		data-tooltip-id="showcase-tooltip"
		data-tooltip-text="Some tooltip text"
		data-tooltip-place="right"
		data-tooltip-type="info"
		tabIndex="0"
	>
		Info tooltip
	</span>
	<span
		data-tooltip-id="showcase-tooltip"
		data-tooltip-text="Some tooltip text"
		data-tooltip-place="bottom"
		data-tooltip-type="warning"
		tabIndex="0"
	>
		Warning tooltip
	</span>
	<span
		data-tooltip-id="showcase-tooltip"
		data-tooltip-text="Some tooltip text"
		data-tooltip-type="error"
		tabIndex="0"
	>
		Error tooltip
	</span>
	<span
		data-tooltip-id="showcase-tooltip"
		data-tooltip-text="Some tooltip text"
		data-tooltip-place="left"
		data-tooltip-type="success"
		tabIndex="0"
	>
		Success tooltip
	</span>
</section>
<Tooltip id="showcase-tooltip"/>
