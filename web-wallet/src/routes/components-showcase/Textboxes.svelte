<script>
	import { Textbox } from "$lib/dusk/components";

	// eslint-disable-next-line max-len
	const multilineInputValue = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Praesent suscipit, eros quis posuere congue, magna nisi finibus diam, non porta erat libero in mauris. Maecenas hendrerit est diam, vitae ultrices urna volutpat quis.";
</script>

<section>
	<Textbox value="Some text here"/>
	<Textbox placeholder="placeholder text"/>
	<Textbox disabled={true} value="Some text here"/>
	<Textbox disabled={true} placeholder="placeholder text"/>
	<Textbox
		type="multiline"
		value={multilineInputValue}/>
</section>
