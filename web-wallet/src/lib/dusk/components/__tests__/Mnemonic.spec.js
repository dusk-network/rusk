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
		const { container, getAllByRole } = render(Mnemonic, {
			props: {
				enteredMnemonicPhrase: enteredSeed,
				mnemonicPhrase: seed,
				type: "validate"
			}
		});

		const buttons = container.querySelectorAll("button[data-value]");

		for (const word of buttons) {
			await fireEvent.click(word);
			expect(word).toBeDisabled();
		}

		const enteredPhrase = getAllByRole("listitem");

		enteredPhrase.forEach((word, index) => {
			expect(word.textContent).toBe(buttons[index].textContent);
		});
	});

	it("should revert the most recent word on Undo click", async () => {
		const { container, getByText, getAllByRole } = render(Mnemonic, {
			props: {
				enteredMnemonicPhrase: enteredSeed,
				mnemonicPhrase: seed,
				type: "validate"
			}
		});

		const buttons = container.querySelectorAll("button[data-value]");

		// Enters the first 5 words
		for (let i = 0; i < 5; i++) {
			await fireEvent.click(buttons[i]);
			expect(buttons[i]).toBeDisabled();
		}

		const undoButton = getByText("Undo");

		// Presses the "Undo button"
		await fireEvent.click(undoButton);

		const enteredPhrase = getAllByRole("listitem");

		// Verify that the content of each list item matches the corresponding button's text content
		// Loop until the 4th item (index 3)
		for (let i = 0; i <= 3; i++) {
			expect(enteredPhrase[i].textContent).toBe(buttons[i].textContent);
		}

		// The 5th item (index 4) and next should be empty
		for (let i = 4; i <= 11; i++) {
			expect(enteredPhrase[i].textContent).toBe("_____");
		}
	});
});
