import { readContract, simulateContract, writeContract } from "@wagmi/core";
import { formatUnits, parseUnits } from "viem";
import ERC20Abi from "./abi/erc_bep_20.json";
import migrationABI from "./abi/migrationABI.json";
import { wagmiConfig } from "./walletConnection";

const stableCoinDecimals = 18;

/**
 * Retrieves the allowance amount for a stable coin for a given spender and account.
 *
 * @param {HexString} userAddress - The address of the user to check if it gave an allowance to the migration contract.
 * @param {HexString} stableCoinAddress - contract address for the coin.
 * @param {HexString} migrationContract - migration contract address for the the coin.
 * @returns {Promise<string>} - A promise that resolves to the allowed amount.
 * @throws {Error} - Throws an error if there is an issue with retrieving the allowance.
 */
export const allowance = async (
  userAddress,
  stableCoinAddress,
  migrationContract
) => {
  try {
    const balance = /** @type {bigint} */ (
      await readContract(wagmiConfig, {
        abi: ERC20Abi,
        address: stableCoinAddress,
        args: [userAddress, migrationContract],
        functionName: "allowance",
      })
    );
    return formatUnits(balance, stableCoinDecimals);
  } catch (e) {
    const errorMessage =
      e instanceof Error
        ? `An error occurred while checking the spender ${e.message}`
        : "An unexpected error occurred while checking the spender";
    throw new Error(errorMessage);
  }
};

/**
 * Approves a spender to transfer a specified amount of stable coin on behalf of the caller.
 *
 * @param {string} value - The amount of stable coin to approve.
 * @param {HexString} stableCoinAddress - contract address for the coin.
 * @param {HexString} migrationContract - migration contract address for the the coin.
 * @returns {Promise<string>} - Transaction hash.
 * @throws {Error} - Throws an error if there is an issue with the approval transaction.
 */
export const approve = async (migrationContract, stableCoinAddress, value) => {
  try {
    const convertedValue = parseUnits(value, stableCoinDecimals);
    return await writeContract(wagmiConfig, {
      abi: ERC20Abi,
      address: stableCoinAddress,
      args: [migrationContract, convertedValue],
      functionName: "approve",
    });
  } catch (e) {
    const errorMessage =
      e instanceof Error
        ? `An error occurred while approving the stable coin ${e.message}`
        : "An unexpected error occurred while approving the stable coin";
    throw new Error(errorMessage);
  }
};

/**
 * Retrieves the balance of the stable coin for a given account.
 *
 * @param {HexString} userAddress - The address of the account to check the balance for.
 * @param {HexString} stableCoinAddress - contract address for the coin.
 * @returns {Promise<bigint>} - A promise that resolves to the balance of the stable coin.
 * @throws {Error} - Throws an error if there is an issue with retrieving the balance.
 */
export const getBalanceOfCoin = async (userAddress, stableCoinAddress) => {
  try {
    const balance = /** @type {bigint} */ (
      await readContract(wagmiConfig, {
        abi: ERC20Abi,
        address: stableCoinAddress,
        args: [userAddress],
        functionName: "balanceOf",
      })
    );
    return balance;
  } catch (e) {
    const errorMessage =
      e instanceof Error
        ? `An error occurred while checking the spender ${e.message}`
        : "An unknown error occurred while checking the spender";
    throw new Error(errorMessage);
  }
};

/**
 *  Migrates the approved amount to the given account
 *
 * @param {string} amount - the amount to be migrated
 * @param {number} chainId - the id of the smart contract
 * @param {string} mainnetDuskAddress - the wallet address where tokens should be migrated to
 * @param {HexString} migrationContract - the migration contract address
 * @returns {Promise<HexString>} - The transaction hash.
 */
export const migrate = async (
  amount,
  chainId,
  mainnetDuskAddress,
  migrationContract
) => {
  const { request } = await simulateContract(wagmiConfig, {
    abi: migrationABI,
    address: migrationContract,
    args: [parseUnits(amount, 18), mainnetDuskAddress],
    chainId: chainId,
    functionName: "migrate",
  });

  return await writeContract(wagmiConfig, request);
};
