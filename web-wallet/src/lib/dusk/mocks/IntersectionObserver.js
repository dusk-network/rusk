export default class IntersectionObserver {
	get root () { return document; }

	get rootMargin () { return "0px 0px 0px 0px"; }

	get thresholds () { return [0]; }

	disconnect () {}

	observe () {}

	takeRecords () {}

	unobserve () {}
}
