/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as web3 from '@solana/web3.js'
import * as beet from '@metaplex-foundation/beet'
import * as beetSolana from '@metaplex-foundation/beet-solana'
import {
  StateAccountType,
  stateAccountTypeBeet,
} from '../types/StateAccountType'
import { BetInput, betInputBeet } from '../types/BetInput'

/**
 * Arguments used to create {@link VrfResult}
 * @category Accounts
 * @category generated
 */
export type VrfResultArgs = {
  accountType: StateAccountType
  isFullfilled: boolean
  isUsed: boolean
  markedForClose: boolean
  owner: web3.PublicKey
  game: web3.PublicKey
  betId: number
  alpha: number[] /* size: 72 */
  beta: number[] /* size: 64 */
  pi: number[] /* size: 80 */
  betInput: BetInput
}
/**
 * Holds the data for the {@link VrfResult} Account and provides de/serialization
 * functionality for that data
 *
 * @category Accounts
 * @category generated
 */
export class VrfResult implements VrfResultArgs {
  private constructor(
    readonly accountType: StateAccountType,
    readonly isFullfilled: boolean,
    readonly isUsed: boolean,
    readonly markedForClose: boolean,
    readonly owner: web3.PublicKey,
    readonly game: web3.PublicKey,
    readonly betId: number,
    readonly alpha: number[] /* size: 72 */,
    readonly beta: number[] /* size: 64 */,
    readonly pi: number[] /* size: 80 */,
    readonly betInput: BetInput
  ) {}

  /**
   * Creates a {@link VrfResult} instance from the provided args.
   */
  static fromArgs(args: VrfResultArgs) {
    return new VrfResult(
      args.accountType,
      args.isFullfilled,
      args.isUsed,
      args.markedForClose,
      args.owner,
      args.game,
      args.betId,
      args.alpha,
      args.beta,
      args.pi,
      args.betInput
    )
  }

  /**
   * Deserializes the {@link VrfResult} from the data of the provided {@link web3.AccountInfo}.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static fromAccountInfo(
    accountInfo: web3.AccountInfo<Buffer>,
    offset = 0
  ): [VrfResult, number] {
    return VrfResult.deserialize(accountInfo.data, offset)
  }

  /**
   * Retrieves the account info from the provided address and deserializes
   * the {@link VrfResult} from its data.
   *
   * @throws Error if no account info is found at the address or if deserialization fails
   */
  static async fromAccountAddress(
    connection: web3.Connection,
    address: web3.PublicKey
  ): Promise<VrfResult> {
    const accountInfo = await connection.getAccountInfo(address)
    if (accountInfo == null) {
      throw new Error(`Unable to find VrfResult account at ${address}`)
    }
    return VrfResult.fromAccountInfo(accountInfo, 0)[0]
  }

  /**
   * Provides a {@link web3.Connection.getProgramAccounts} config builder,
   * to fetch accounts matching filters that can be specified via that builder.
   *
   * @param programId - the program that owns the accounts we are filtering
   */
  static gpaBuilder(
    programId: web3.PublicKey = new web3.PublicKey(
      'HiEuiREGdSuBYv4oxtdkWnYtcnNUKk8m93XSn8pPYtcm'
    )
  ) {
    return beetSolana.GpaBuilder.fromStruct(programId, vrfResultBeet)
  }

  /**
   * Deserializes the {@link VrfResult} from the provided data Buffer.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static deserialize(buf: Buffer, offset = 0): [VrfResult, number] {
    return vrfResultBeet.deserialize(buf, offset)
  }

  /**
   * Serializes the {@link VrfResult} into a Buffer.
   * @returns a tuple of the created Buffer and the offset up to which the buffer was written to store it.
   */
  serialize(): [Buffer, number] {
    return vrfResultBeet.serialize(this)
  }

  /**
   * Returns the byteSize of a {@link Buffer} holding the serialized data of
   * {@link VrfResult} for the provided args.
   *
   * @param args need to be provided since the byte size for this account
   * depends on them
   */
  static byteSize(args: VrfResultArgs) {
    const instance = VrfResult.fromArgs(args)
    return vrfResultBeet.toFixedFromValue(instance).byteSize
  }

  /**
   * Fetches the minimum balance needed to exempt an account holding
   * {@link VrfResult} data from rent
   *
   * @param args need to be provided since the byte size for this account
   * depends on them
   * @param connection used to retrieve the rent exemption information
   */
  static async getMinimumBalanceForRentExemption(
    args: VrfResultArgs,
    connection: web3.Connection,
    commitment?: web3.Commitment
  ): Promise<number> {
    return connection.getMinimumBalanceForRentExemption(
      VrfResult.byteSize(args),
      commitment
    )
  }

  /**
   * Returns a readable version of {@link VrfResult} properties
   * and can be used to convert to JSON and/or logging
   */
  pretty() {
    return {
      accountType: 'StateAccountType.' + StateAccountType[this.accountType],
      isFullfilled: this.isFullfilled,
      isUsed: this.isUsed,
      markedForClose: this.markedForClose,
      owner: this.owner.toBase58(),
      game: this.game.toBase58(),
      betId: this.betId,
      alpha: this.alpha,
      beta: this.beta,
      pi: this.pi,
      betInput: this.betInput.__kind,
    }
  }
}

/**
 * @category Accounts
 * @category generated
 */
export const vrfResultBeet = new beet.FixableBeetStruct<
  VrfResult,
  VrfResultArgs
>(
  [
    ['accountType', stateAccountTypeBeet],
    ['isFullfilled', beet.bool],
    ['isUsed', beet.bool],
    ['markedForClose', beet.bool],
    ['owner', beetSolana.publicKey],
    ['game', beetSolana.publicKey],
    ['betId', beet.u32],
    ['alpha', beet.uniformFixedSizeArray(beet.u8, 72)],
    ['beta', beet.uniformFixedSizeArray(beet.u8, 64)],
    ['pi', beet.uniformFixedSizeArray(beet.u8, 80)],
    ['betInput', betInputBeet],
  ],
  VrfResult.fromArgs,
  'VrfResult'
)
