type ContractDescriptor = {
	disabled: boolean;
	id: string;
	label: string;
	operations: ContractOperation[];
};

type ContractGasSettings = {
	gasLimit: number;
	gasPrice: number;
};

type ContractOperation = {
	disabled: boolean;
	id: string;
	label: string;
	primary: boolean;
};

type ContractStatus = {
	label: string;
	value: string;
};

type StakeType = "stake" | "unstake" | "withdraw-rewards";
