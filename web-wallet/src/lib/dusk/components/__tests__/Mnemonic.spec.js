import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import {
	cleanup,
	fireEvent,
	render
} from "@testing-library/svelte";
import { Mnemonic } from "..";

/** @type {string[]} */
const enteredSeed = [];

/** @type {string[]} */
const seed = ["auction", "tribe", "type", "torch", "domain", "auction",
	"lyrics", "mouse", "alert", "fabric", "snake", "ticket"];

describe("Mnemonic", () => {
	afterEach(cleanup);

	it("should render the \"Mnemonic\" component in the authenticate state", async () => {
		const { container } = render(Mnemonic, {
			props: {
				enteredMnemonicPhrase: enteredSeed,
				type: "authenticate"
			}
		});

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the \"Mnemonic\" component in the validate state", () => {
		const { container } = render(Mnemonic, {
			props: {
				mnemonicPhrase: seed,
				type: "validate"
			}
		});

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should display all the words in the order they have been clicked", async () => {
		const { getAllByRole } = render(Mnemonic, {
			props: {
				enteredMnemonicPhrase: enteredSeed,
				mnemonicPhrase: seed,
				type: "validate"
			}
		});

		const buttons = getAllByRole("button");

		for (const word of buttons) {
			await fireEvent.click(word);
			expect(word).toBeDisabled();
		}

		const enteredPhrase = getAllByRole("listitem");

		enteredPhrase.forEach((word, index) => {
			expect(word.textContent).toBe(buttons[index].textContent);
		});
	});
});
