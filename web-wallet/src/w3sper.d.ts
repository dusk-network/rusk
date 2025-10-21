declare module "@dusk/w3sper" {
  /***** UTILITY TYPES *****/

  type BiMapEnum<T extends string[], I extends number = 0> = T extends [
    infer First extends string,
    ...infer Rest extends string[],
  ]
    ? { [K in Uppercase<First>]: BiMapValue<First, I> } & BiMapEnum<
        Rest,
        Increment<I>
      >
    : {};

  type BiMapValue<V extends string, I extends number> = {
    [Symbol.toPrimitive]<T extends ToPrimitiveHint>(
      hint: T
    ): T extends "number" ? I : T extends "string" ? V : BiMapValue<V, I>;

    toString(): V;
  };

  type Chains = ["localnet", "mainnet", "testnet", "devnet", "unknown"];

  type ChainsEnum = BiMapEnum<Chains>;

  type ChainValue = ChainsEnum[Uppercase<Chains[number]>];

  type Increment<
    N extends number,
    Arr extends unknown[] = [],
  > = Arr["length"] extends N
    ? [...Arr, unknown]["length"]
    : Increment<N, [...Arr, unknown]>;

  type ShieldedTransferResult = Readonly<{
    buffer: Uint8Array;
    hash: string;
    nullifiers: Uint8Array[];
  }>;

  type ToPrimitiveHint = "default" | "number" | "string";

  type UnshieldedTransferResult = Readonly<{
    buffer: Uint8Array;
    hash: string;
    nonce: bigint;
  }>;

  type WithSeeder = Key | Profile | ProfileGenerator;

  /***** CONSTANTS *****/

  const TRANSFER =
    "0100000000000000000000000000000000000000000000000000000000000000";

  /***** FUNCTIONS *****/

  function useAsProtocolDriver(
    source: string | URL | Uint8Array,
    importsURL?: URL
  ): PromiseLike<void> & {
    cleanup: Promise<void>;
  };

  /***** INTERFACES *****/

  interface AccountTransfer extends Transfer {
    get attributes(): Transfer["attributes"] & {
      chain: ChainValue;
      nonce: bigint;
    };

    build(network?: Network): Promise<UnshieldedTransferResult>;

    chain(value: ChainValue): this;

    nonce(value: bigint): this;
  }

  interface AddressTransfer extends Transfer {
    get attributes(): Transfer["attributes"] & {
      chain: ChainValue;
      obfuscated: boolean;
    };

    build(network: Network): Promise<ShieldedTransferResult>;

    chain(value: ChainValue): this;

    obfuscated(): this;
  }

  abstract class BasicTransfer {
    constructor(from: BookEntry | Profile);

    get attributes(): {
      amount: bigint;
      gas: Gas;
    };

    readonly bookentry: BookEntry | { profile: Profile };

    amount(value: bigint): this;

    build(
      network?: Network
    ): Promise<ShieldedTransferResult | UnshieldedTransferResult>;

    gas(value?: GasValues | null): this;
  }

  interface Blocks extends RuesScope {
    get gasPrice(): {
      average: bigint;
      max: bigint;
      median: bigint;
      min: bigint;
    };
  }

  interface Contracts extends RuesScope {
    get stakeContract(): RuesTarget;
    get transferContract(): RuesTarget;
  }

  abstract class Rues extends EventTarget {
    static connect(
      url: string | URL,
      options?: { signal: AbortSignal }
    ): Promise<string>;

    get connected(): boolean;
    get sessionId(): Promise<Record<string, any>>;
    get url(): URL;
    get version(): string;

    connect(options?: { signal: AbortSignal }): Promise<this>;

    disconnect(): Promise<void>;

    handleEvent(event: MessageEvent): void;

    scope(source: string | RuesScope): Readonly<RuesTarget>;
  }

  abstract class RuesEvent extends Event {
    static from(
      event: MessageEvent | RuesEvent,
      options?: { as: "component" | "string" }
    ): RuesEvent;

    get headers(): Headers;
    get origin(): RuesEventOrigin;
    get payload(): Uint8Array | Record<string, any> | unknown;
  }

  abstract class RuesScope {
    constructor(name: string);

    get call(): any;
    get on(): any;
    get once(): any;

    name: string;

    eventFrom(ruesEvent: RuesEvent): RuesEvent;

    withId(id: string): RuesTarget;
  }

  abstract class StakeInfo {
    static parse(buffer: ArrayBuffer): Readonly<StakeInfo>;

    amount: StakeAmount | null;
    faults: number;
    hardFaults: number;
    reward: bigint;
  }

  interface TransactionExecutedEvent extends RuesEvent {
    get gasPaid(): bigint;

    memo(options?: {
      as: "string";
    }): typeof options extends undefined ? Uint8Array : string;
  }

  interface Transactions extends RuesScope {
    eventFrom<T extends MessageEvent | RuesEvent>(
      ruesEvent: T
    ): T extends RuesEvent
      ? T["origin"]["topic"] extends "executed"
        ? TransactionExecutedEvent
        : T
      : T;

    preverify<T extends BasicTransfer>(tx: T): Promise<T>;

    propagate<T extends BasicTransfer>(tx: T): Promise<T>;
  }

  interface Treasury {
    account(identifier: Key): Promise<AccountBalance>;
    address(identifier: Key): Promise<Map<Uint8Array, Uint8Array>>;
    stakeInfo(identifier: Key): Promise<StakeInfo>;
  }

  /***** MISC TYPES *****/

  type AccountBalance = {
    nonce: bigint;
    value: bigint;
  };

  type AddressBalance = {
    spendable: bigint;
    value: bigint;
  };

  type BookEntry = Readonly<{
    get info(): {
      balance<T extends "account" | "address">(
        type: T
      ): Promise<T extends "account" ? AccountBalance : AddressBalance>;

      stake(): Promise<StakeInfo>;
    };

    bookkeeper: Bookkeeper;
    profile: Profile;

    contract(contractId: string, network: Network): Contract;

    shield(amount: bigint): ShieldTransfer;

    stake(amount: bigint): StakeTransfer;

    topup(amount: bigint): StakeTransfer;

    transfer(amount: bigint): Transfer;

    unshield(amount: bigint): UnshieldTransfer;

    unstake(amount: bigint): UnstakeTransfer;

    withdraw(amount: bigint): WithdrawStakeRewardTransfer;
  }>;

  type GasValues = {
    limit: bigint | number;
    price: bigint | number;
  };

  type Key = {
    [Symbol.toPrimitive]<T extends ToPrimitiveHint>(
      hint: T
    ): T extends "number" ? number : T extends "string" ? string : null;

    get seed(): Uint8Array;

    toString(): string;

    valueOf(): Uint8Array;
  };

  type Provisioner = {
    amount: number;
    eligibility: number;
    faults: number;
    hard_faults: number;
    key: string;
    locked_amt: number;
    owner: { Account: string } | { Contract: string };
    reward: number;
  };

  type RuesEventOrigin = Readonly<{
    id: string;
    scope: string;
    topic: string;

    toString(): string;
  }>;

  type RuesTarget = {
    get rues(): Rues;

    id: string;
    options: Record<string, any>;
    scope: RuesScope;

    get call(): RuesScope["call"];
    get on(): RuesScope["on"];
    get once(): RuesScope["once"];

    toString(): string;
    toURL(): URL;
    withId(id: string): Readonly<RuesTarget>;
  };

  type StakeAmount = {
    get total(): bigint;

    eligibility: bigint;
    locked: bigint;
    value: bigint;
  };

  /***** PUBLIC CLASSES *****/

  class AccountSyncer extends EventTarget {
    constructor(network: Network);

    balances(profiles: Profile[]): Promise<AccountBalance[]>;

    stakes(profiles: Profile[]): Promise<Readonly<StakeInfo>[]>;
  }

  class AddressSyncer extends EventTarget {
    constructor(network: Network);

    get root(): Promise<ArrayBuffer>;

    // The `signal` in the options isn't in w3sper yet
    notes(
      profiles: Profile[],
      options?: { from?: bigint | Bookmark; signal?: AbortSignal }
    ): Promise<
      ReadableStream<
        [
          Array<Map<Uint8Array, Uint8Array>>,
          {
            blockHeight: bigint;
            bookmark: bigint;
          },
        ]
      >
    >;

    openings(notes: Map<Uint8Array, Uint8Array>): Promise<ArrayBuffer[]>;

    spent(nullifiers: Uint8Array[]): Promise<ArrayBuffer[]>;
  }

  class Bookkeeper {
    constructor(treasury: Treasury);

    get minimumStake(): Promise<bigint>;

    as(profile: Profile): BookEntry;

    balance(identifier: Key): Promise<AccountBalance | AddressBalance>;

    pick(identifier: Key, amount: bigint): Promise<Map<Uint8Array, Uint8Array>>;

    stakeInfo(identifier: Key): Promise<StakeInfo>;
  }

  class Bookmark {
    constructor(data: Uint8Array);

    static from(source: bigint | number): Bookmark;

    get data(): Uint8Array;

    asUint(): bigint;

    isNone(): boolean;

    toString(): string;
  }

  class Gas {
    constructor(values?: GasValues | null);

    static readonly DEFAULT_LIMIT: bigint;
    static readonly DEFAULT_PRICE: bigint;

    readonly limit: bigint;
    readonly price: bigint;
    readonly total: bigint;
  }

  class Network extends EventTarget {
    constructor(url: string | URL, options?: {});

    static DEVNET: ChainsEnum["DEVNET"];
    static LOCALNET: ChainsEnum["LOCALNET"];
    static MAINNET: ChainsEnum["MAINNET"];
    static TESTNET: ChainsEnum["TESTNET"];

    static connect(
      url: string | URL,
      options?: { signal: AbortSignal }
    ): Promise<Network>;

    get blockHeight(): Promise<bigint>;

    get connected(): boolean;

    get rues(): Rues;

    get url(): URL;

    blocks: Blocks;

    contracts: Contracts;

    dataDrivers: DataDriverRegistry;

    node: Node;

    transactions: Transactions;

    connect(options?: { signal: AbortSignal }): Promise<this>;

    disconnect(): Promise<void>;

    execute<T extends BasicTransfer>(tx: T): ReturnType<T["build"]>;

    prove(circuits: Uint8Array): Promise<ArrayBuffer>;

    query(
      gql?: string,
      options?: Record<string, any>
    ): Promise<Record<string, any>>;
  }

  class DataDriverRegistry {
    constructor(fetch: (url: string | URL) => Promise<ArrayBuffer>);

    register(key: string, locator: any): DataDriverRegistry;

    has(key: string): boolean;

    get(key: string): Promise<WebAssembly.Module | null>;
  }

  class Node {
    constructor(rues: Rues);

    static CHAIN: ChainsEnum;

    get info(): Promise<{
      bootstrappingNodes: string[];
      chain: ChainValue;
      chainId: number;
      kadcastAddress: string;
      version: string;
      versionBuild: string;
    }>;

    crs(): Promise<ArrayBuffer>;

    provisioners(): Promise<Provisioner[]>;
  }

  class Profile {
    constructor(buffer: Uint8Array);

    [Symbol.toPrimitive]<T extends ToPrimitiveHint>(
      hint: T
    ): T extends "number" ? number : null;

    get account(): Key;

    get address(): Key;

    get seed(): Uint8Array;

    sameSourceOf(profile: Profile): boolean;
  }

  class ProfileGenerator {
    constructor(seeder: () => Promise<Uint8Array>);
    constructor(seeder: () => Uint8Array);

    get default(): Promise<Profile>;

    get length(): number;

    static seedFrom<T>(
      target: T
    ): T extends WithSeeder ? Uint8Array : undefined;

    static typeOf(value: string): "account" | "address" | "undefined";

    at(index: number): Promise<Profile>;

    indexOf(profile: Profile): number;

    next(): Promise<Profile>;
  }

  class ShieldTransfer extends BasicTransfer {
    build(network: Network): Promise<UnshieldedTransferResult>;
  }

  class StakeTransfer extends BasicTransfer {
    constructor(from: BookEntry | Profile, options?: { topup: boolean });

    get attributes(): BasicTransfer["attributes"] & { topup: boolean };

    build(network: Network): Promise<UnshieldedTransferResult>;
  }

  class Transfer extends BasicTransfer {
    get attributes(): BasicTransfer["attributes"] & { to: string };

    to(value: string | Key): AccountTransfer | AddressTransfer;

    memo(value: Uint8Array | string): this;

    deposit(value: bigint): this;
  }

  class UnshieldTransfer extends BasicTransfer {
    build(network: Network): Promise<ShieldedTransferResult>;
  }

  class UnstakeTransfer extends BasicTransfer {
    build(network: Network): Promise<UnshieldedTransferResult>;
  }

  class WithdrawStakeRewardTransfer extends BasicTransfer {
    build(network: Network): Promise<UnshieldedTransferResult>;
  }

  interface ContractDriver {
    getSchema?(): unknown | Promise<unknown>;
    getVersion?(): unknown | Promise<unknown>;

    encodeInputFn(
      fnName: string,
      json: string
    ): Uint8Array | Promise<Uint8Array>;

    decodeOutputFn(
      fnName: string,
      bytes: Uint8Array
    ): unknown | Promise<unknown>;

    decodeEvent(name: string, bytes: Uint8Array): unknown | Promise<unknown>;
  }

  type ContractCallOptions = {
    feeder?: boolean;
    [key: string]: unknown;
  };

  type ContractPayload = Readonly<{
    fnName: string;
    fnArgs: Uint8Array;
    contractId: number[];
  }>;

  type ContractTransferBuilder = (
    | Transfer
    | AccountTransfer
    | AddressTransfer
  ) & {
    payload(p: ContractPayload): ContractTransferBuilder;
  };

  type ContractConstructorParams = Readonly<{
    contractId: string | Uint8Array;
    driver: ContractDriver | Promise<ContractDriver>;
    network?: Network | null;
    bookentry?: BookEntry | null;
  }>;

  class Contract {
    constructor(params: ContractConstructorParams);

    get id(): string;

    schema(): Promise<unknown>;
    version(): Promise<unknown>;

    encode(
      fnName: string | number | symbol,
      jsonValue?: unknown
    ): Promise<Uint8Array>;

    readonly call: {
      [fnName: string]: (
        args?: unknown,
        options?: ContractCallOptions
      ) => Promise<any>;
    };

    readonly tx: {
      [fnName: string]: (args?: unknown) => Promise<ContractTransferBuilder>;
    };

    readonly events: {
      [eventName: string]: {
        once(): Promise<unknown>;
        on(handler: (data?: unknown, error?: unknown) => void): () => void;
      };
    };
  }
}
