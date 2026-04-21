/**
 * Type declarations for the public `@dusk/w3sper` API.
 */

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export interface JsonObject {
  [key: string]: JsonValue;
}

export type ContractValue =
  | JsonValue
  | bigint
  | Uint8Array
  | ArrayBuffer
  | ContractObject
  | ContractValue[];

export interface ContractObject {
  [key: string]: ContractValue;
}

export interface RuesEventOrigin {
  readonly scope: string;
  readonly id?: string;
  readonly topic: string;
  toString(): string;
}

export interface RuesEvent extends Event {
  readonly headers: Headers;
  readonly payload: JsonValue | Uint8Array;
  readonly origin: RuesEventOrigin;
}

export interface RuesListenOptions extends RequestInit {
  signal?: AbortSignal;
}

export interface RuesCallOptions extends RequestInit {
  feeder?: boolean;
}

export interface LooseOptions {
  [key: string]:
    | JsonValue
    | bigint
    | Uint8Array
    | ArrayBuffer
    | undefined;
}

export interface ScopedRuesCall {
  [method: string]: (
    body?: BodyInit | Uint8Array | ArrayBuffer | null,
    options?: RuesCallOptions,
  ) => Promise<Response>;
}

export interface ScopedRuesOn {
  [topic: string]: (
    listener: (event: RuesEvent) => void,
    options?: RuesListenOptions,
  ) => Promise<void>;
}

export interface ScopedRuesOnce {
  [topic: string]: (options?: RuesListenOptions) => Promise<RuesEvent>;
}

export interface ScopedRuesTarget {
  readonly call: ScopedRuesCall;
  readonly on: ScopedRuesOn;
  readonly once: ScopedRuesOnce;
  withId(id: string | Uint8Array): ScopedRuesTarget;
}

/**
 * Converts between Lux units (`bigint`) and human-friendly Dusk decimal strings.
 */
export namespace lux {
  /**
   * Formats a Lux amount into Dusk decimal notation.
   */
  function formatToDusk(luxValue: bigint): string;

  /**
   * Parses a Dusk decimal string into Lux units.
   */
  function parseFromDusk(duskValue: string): bigint;
}

export interface WasmDataDriver {
  init(): void;
  encodeInputFn(fnName: string, json: string): Uint8Array;
  decodeInputFn(fnName: string, rkyvBytes: Uint8Array): JsonValue;
  decodeOutputFn(fnName: string, rkyvBytes: Uint8Array): JsonValue;
  decodeEvent(eventName: string, rkyvBytes: Uint8Array): JsonValue;
  getSchema(): JsonValue;
  getVersion(): string;
}

export type DataDriverLocator =
  | string
  | URL
  | ArrayBuffer
  | Uint8Array
  | (() =>
    | Promise<ArrayBuffer | Uint8Array>
    | ArrayBuffer
    | Uint8Array);

/**
 * Loads and tracks contract data-driver WASM artifacts.
 */
export namespace dataDrivers {
  /**
   * Loads a data-driver WASM payload.
   */
  function load(
    source: string | URL | ArrayBuffer | Uint8Array,
  ): Promise<WasmDataDriver>;

  /**
   * Registry used to cache and resolve contract data-drivers by contract id.
   */
  class DataDriverRegistry {
    constructor(fetchImpl?: typeof fetch);
    register(
      contractId: string | Uint8Array,
      driver: DataDriverLocator,
    ): this;
    has(contractId: string | Uint8Array): boolean;
    get(
      contractId: string | Uint8Array,
      value?: DataDriverLocator,
    ): Promise<WasmDataDriver>;
  }
}

/**
 * Cursor used for paged note synchronization.
 */
export class Bookmark {
  constructor(data: Uint8Array);
  static from(
    source: Bookmark | bigint | number | Uint8Array | ArrayLike<number>,
  ): Bookmark;
  get data(): Uint8Array;
  asUint(): bigint;
  toString(): string;
  isNone(): boolean;
}

export interface AccountBalance {
  nonce: bigint;
  value: bigint;
}

export interface AddressBalance {
  value: bigint;
  spendable: bigint;
}

export interface StakeAmount {
  value: bigint;
  locked: bigint;
  eligibility: bigint;
  readonly total: bigint;
}

export interface StakeInfo {
  amount: StakeAmount | null;
  reward: bigint;
  faults: number;
  hardFaults: number;
}

export interface SyncInfo {
  bookmark: bigint;
  blockHeight: bigint;
}

export interface SyncIterationDetail {
  ownedCount: number;
  progress: number;
  bookmarks: {
    current: bigint;
    last: bigint;
  };
  blocks: {
    current: bigint;
    last: bigint;
  };
}

export type OwnedNotesBatch = Array<Map<Uint8Array, Uint8Array>>;

/**
 * Syncs Phoenix notes and nullifier state for one or more profiles.
 */
export class AddressSyncer extends EventTarget {
  constructor(network: Network, options?: LooseOptions);
  get root(): Promise<ArrayBuffer>;
  openings(
    notes: Map<ArrayBuffer | Uint8Array, Uint8Array> | Uint8Array[],
    options?: LooseOptions,
  ): Promise<ArrayBuffer[]>;
  spent(nullifiers: ArrayBuffer[] | Uint8Array[]): Promise<ArrayBuffer[]>;
  notes(
    profiles: Profile[],
    options?: { from?: bigint | Bookmark },
  ): Promise<ReadableStream<[OwnedNotesBatch, SyncInfo]>>;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: "synciteration",
    listener: (event: CustomEvent<SyncIterationDetail>) => void,
    options?: boolean | AddEventListenerOptions,
  ): void;
}

export interface AccountHistoryEntry {
  blockHash: string;
  blockHeight: bigint;
  blockTimestamp: number;
  feePaid: bigint;
  from: string;
  gasLimit: bigint;
  gasPrice: bigint;
  gasSpent: bigint;
  hash: string;
  memo: string;
  method: string;
  owner: string;
  success: boolean;
  to: string;
  type: "public";
  value: bigint;
}

/**
 * Syncs account balance, nonce, stake and account history.
 */
export class AccountSyncer extends EventTarget {
  constructor(network: Network);
  balances(
    profiles: Array<Profile | AccountKey | string>,
  ): Promise<AccountBalance[]>;
  history(
    profiles: Profile[],
    options?: {
      from?: bigint;
      to?: bigint;
      limit?: number;
      order?: "asc" | "desc";
      signal?: AbortSignal;
    },
  ): Promise<Array<ReadableStream<AccountHistoryEntry>>>;
  stakes(profiles: Array<Profile | AccountKey | string>): Promise<StakeInfo[]>;
}

/**
 * Fee configuration for transactions.
 */
export class Gas {
  constructor(value?: { limit?: bigint | number; price?: bigint | number });
  readonly limit: bigint;
  readonly price: bigint;
  readonly total: bigint;
}

export interface NodeChainValue {
  toString(): string;
}

export interface NodeInfo {
  chainId: number;
  chain: NodeChainValue;
  [key: string]: JsonValue | NodeChainValue;
}

export interface NodeComponent {
  readonly info: Promise<NodeInfo>;
  crs(): Promise<ArrayBuffer>;
  provisioners(): Promise<JsonValue>;
}

export interface BlocksComponent extends ScopedRuesTarget {
  readonly gasPrice: Promise<Record<string, bigint>>;
}

export interface TransactionExecutedEvent extends RuesEvent {
  readonly gasPaid: bigint;
  memo(options?: { as?: "string" }): string | Uint8Array | null;
  call(): JsonValue;
}

export type TransactionBytesLike =
  | Uint8Array
  | ArrayBuffer
  | DataView
  | { valueOf(): BodyInit };

export interface TransactionsComponent extends ScopedRuesTarget {
  preverify<T extends TransactionBytesLike>(tx: T): Promise<T>;
  propagate<T extends TransactionBytesLike>(tx: T): Promise<T>;
  eventFrom(ruesEvent: RuesEvent): RuesEvent | TransactionExecutedEvent;
}

export interface ContractsComponent extends ScopedRuesTarget {
  withId(id: string | Uint8Array): ScopedRuesTarget;
  readonly transferContract: ScopedRuesTarget;
  readonly stakeContract: ScopedRuesTarget;
}

export interface NetworkConstructorOptions {
  version?: string;
  [key: string]: JsonValue | undefined;
}

export interface NetworkConnectOptions {
  signal?: AbortSignal;
}

export interface GraphQLRequest {
  query: string;
  variables?: JsonObject;
  [key: string]: JsonValue | undefined;
}

/**
 * Main network client for interacting with a Rusk node.
 */
export class Network extends EventTarget {
  static readonly LOCALNET: string;
  static readonly MAINNET: string;
  static readonly TESTNET: string;
  static readonly DEVNET: string;

  static connect(
    url: string | URL,
    options?: NetworkConnectOptions,
  ): Promise<Network>;

  constructor(url: string | URL, options?: NetworkConstructorOptions);
  get url(): URL;
  get connected(): boolean;
  get blockHeight(): Promise<bigint>;

  connect(options?: NetworkConnectOptions): Promise<this>;
  disconnect(): Promise<void>;
  execute(tx: BasicTransfer | BuiltTransaction): Promise<BuiltTransaction>;
  prove(circuits: Uint8Array): Promise<ArrayBuffer>;
  query(
    gql: string | GraphQLRequest,
    options?: RequestInit,
  ): Promise<JsonObject>;

  readonly dataDrivers: dataDrivers.DataDriverRegistry;
  readonly node: NodeComponent;
  readonly contracts: ContractsComponent;
  readonly blocks: BlocksComponent;
  readonly transactions: TransactionsComponent;

  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: "connect" | "disconnect",
    listener: (event: Event) => void,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: "error",
    listener: (event: ErrorEvent) => void,
    options?: boolean | AddEventListenerOptions,
  ): void;
}

declare const profileKeyBrand: unique symbol;
declare const accountKeyBrand: unique symbol;
declare const addressKeyBrand: unique symbol;

export interface ProfileKey {
  toString(): string;
  valueOf(): Uint8Array;
  readonly seed: Promise<ArrayBuffer> | undefined;
  readonly [profileKeyBrand]: true;
}

export interface AccountKey extends ProfileKey {
  readonly [accountKeyBrand]: true;
}

export interface AddressKey extends ProfileKey {
  readonly [addressKeyBrand]: true;
}

/**
 * Address/account keypair derived from a single deterministic seed source.
 */
export class Profile {
  get account(): AccountKey;
  get address(): AddressKey;
  get seed(): Promise<ArrayBuffer> | undefined;
  sameSourceOf(profile: Profile): boolean;
}

/**
 * Deterministic profile generator backed by a seed provider.
 */
export class ProfileGenerator {
  constructor(seeder: () => Promise<ArrayBuffer | Uint8Array>);
  next(): Promise<Profile>;
  get default(): Promise<Profile>;
  at(index: number): Promise<Profile> | undefined;
  indexOf(profile: Profile): number;
  get length(): number;

  static typeOf(value: string): "address" | "account" | "undefined";
  static seedFrom(
    target: Profile | ProfileKey | string | null | undefined,
  ): Promise<ArrayBuffer> | undefined;
}

export type Identifier = string | AccountKey | AddressKey;
export type AccountIdentifier = string | Profile | AccountKey;
export type StakeIdentifier = string | AccountKey;
export type AddressIdentifier = string | AddressKey;

export interface BookEntryInfo {
  balance(type: "account"): Promise<AccountBalance>;
  balance(type: "address"): Promise<AddressBalance>;
  stake(): Promise<StakeInfo>;
}

export interface TreasuryLike {
  account(identifier: AccountIdentifier): Promise<AccountBalance>;
  address(identifier: AddressIdentifier): Promise<Map<Uint8Array, Uint8Array>>;
  stakeInfo(identifier: StakeIdentifier): Promise<StakeInfo>;
}

export interface ContractEvents {
  once(): Promise<JsonValue | Uint8Array | null>;
  on(
    handler: (
      payload: JsonValue | Uint8Array | null | undefined,
      error?: Error,
    ) => void,
  ): () => void;
}

export interface ContractOptions {
  contractId: string | Uint8Array;
  driver: WasmDataDriver | PromiseLike<WasmDataDriver>;
  network?: Network | null;
  bookentry?: BookEntry | null;
}

/**
 * Contract helper that wraps encoded calls/tx builders and decoded events.
 */
export class Contract {
  constructor(options: ContractOptions);
  readonly id: string;
  schema(): Promise<JsonValue>;
  version(): Promise<string>;
  encode(
    fnName: string,
    jsonValue?: ContractValue | null,
  ): Promise<Uint8Array>;
  readonly call: Record<
    string,
    (args?: ContractValue, options?: RuesCallOptions) => Promise<JsonValue>
  >;
  readonly tx: Record<string, (args?: ContractValue) => Promise<Transfer>>;
  readonly events: Record<string, ContractEvents>;
}

export interface BookEntry {
  readonly profile: Profile;
  readonly bookkeeper: Bookkeeper;
  readonly info: BookEntryInfo;
  transfer(amount: bigint): Transfer;
  unshield(amount: bigint): UnshieldTransfer;
  shield(amount: bigint): ShieldTransfer;
  stake(amount: bigint): StakeTransfer;
  unstake(amount: bigint): UnstakeTransfer;
  withdraw(amount: bigint): WithdrawStakeRewardTransfer;
  topup(amount: bigint): StakeTransfer;
  contract(
    contractId: string | Uint8Array,
    network: Network,
    driver?: WasmDataDriver | PromiseLike<WasmDataDriver>,
  ): Contract;
}

export interface AccountTransfer extends Transfer {
  chain(value: number | bigint): this;
  nonce(value: bigint): this;
  build(network: Network): Promise<NonceTransactionResult>;
}

export interface AddressTransfer extends Transfer {
  obfuscated(): this;
  build(network: Network): Promise<NullifierTransactionResult>;
}

/**
 * Helper that builds transfer/stake transactions for a synced treasury.
 */
export class Bookkeeper {
  constructor(treasury: TreasuryLike);
  balance(identifier: AccountIdentifier): Promise<AccountBalance>;
  balance(identifier: AddressIdentifier): Promise<AddressBalance>;
  balance(identifier: Identifier): Promise<AccountBalance | AddressBalance>;
  get minimumStake(): Promise<bigint>;
  stakeInfo(identifier: StakeIdentifier): Promise<StakeInfo>;
  pick(
    identifier: AddressIdentifier,
    amount: bigint,
  ): Promise<Map<Uint8Array, Uint8Array>>;
  as(profile: Profile): BookEntry;
}

/**
 * Canonical transfer contract id used on Dusk networks.
 */
export const TRANSFER:
  "0100000000000000000000000000000000000000000000000000000000000000";

export type NonceTransactionResult = Readonly<{
  buffer: Uint8Array;
  hash: string;
  nonce: bigint;
}>;

export type NullifierTransactionResult = Readonly<{
  buffer: Uint8Array;
  hash: string;
  nullifiers: Uint8Array[];
}>;

export type BuiltTransaction =
  | NonceTransactionResult
  | NullifierTransactionResult;

export abstract class BasicTransfer {
  constructor(from: BookEntry | Profile);
  amount(value: bigint): this;
  gas(value: { limit?: bigint | number; price?: bigint | number }): this;
  build(network: Network): Promise<BuiltTransaction>;
}

/**
 * Base transfer builder that dispatches to account or address flow via `.to(...)`.
 */
export class Transfer extends BasicTransfer {
  constructor(from: BookEntry | Profile);
  deposit(value: bigint): this;
  memo(value: ContractValue): this;
  payload(payload: ContractValue): this;
  to(
    value: string | AccountKey | AddressKey,
  ): AccountTransfer | AddressTransfer;
}

/**
 * Phoenix to Moonlight transfer builder.
 */
export class UnshieldTransfer extends BasicTransfer {
  constructor(from: BookEntry | Profile);
  build(network: Network): Promise<NullifierTransactionResult>;
}

/**
 * Moonlight to Phoenix transfer builder.
 */
export class ShieldTransfer extends BasicTransfer {
  constructor(from: BookEntry | Profile);
  build(network: Network): Promise<NonceTransactionResult>;
}

/**
 * Stake/top-up transfer builder.
 */
export class StakeTransfer extends BasicTransfer {
  constructor(
    from: BookEntry | Profile,
    options?: { topup?: boolean },
  );
  build(network: Network): Promise<NonceTransactionResult>;
}

/**
 * Unstake transfer builder.
 */
export class UnstakeTransfer extends BasicTransfer {
  constructor(from: BookEntry | Profile);
  build(network: Network): Promise<NonceTransactionResult>;
}

/**
 * Stake reward withdrawal transfer builder.
 */
export class WithdrawStakeRewardTransfer extends BasicTransfer {
  constructor(from: BookEntry | Profile);
  build(network: Network): Promise<NonceTransactionResult>;
}

/**
 * Scoped helper that loads a protocol driver and automatically unloads it.
 */
export function useAsProtocolDriver(
  source: string | URL | ArrayBuffer | Uint8Array,
  importsURL?: URL,
): PromiseLike<void> & { cleanup(): Promise<void> };
