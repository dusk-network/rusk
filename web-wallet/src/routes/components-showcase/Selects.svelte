<script>
	import { Select } from "$lib/dusk/components";

	const someOptions = ["apple", "banana", "lemon", "a really big orange", "pear"];
</script>

<section>
	<Select options={someOptions}/>
	<Select disabled={true} options={someOptions}/>
</section>
