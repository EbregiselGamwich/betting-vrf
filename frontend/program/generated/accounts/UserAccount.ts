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

/**
 * Arguments used to create {@link UserAccount}
 * @category Accounts
 * @category generated
 */
export type UserAccountArgs = {
  accountType: StateAccountType
  authority: web3.PublicKey
  totalBets: number
  currentLamports: beet.bignum
  activeVrfResults: number
  referral: beet.COption<web3.PublicKey>
  username: beet.COption<string>
}
/**
 * Holds the data for the {@link UserAccount} Account and provides de/serialization
 * functionality for that data
 *
 * @category Accounts
 * @category generated
 */
export class UserAccount implements UserAccountArgs {
  private constructor(
    readonly accountType: StateAccountType,
    readonly authority: web3.PublicKey,
    readonly totalBets: number,
    readonly currentLamports: beet.bignum,
    readonly activeVrfResults: number,
    readonly referral: beet.COption<web3.PublicKey>,
    readonly username: beet.COption<string>
  ) {}

  /**
   * Creates a {@link UserAccount} instance from the provided args.
   */
  static fromArgs(args: UserAccountArgs) {
    return new UserAccount(
      args.accountType,
      args.authority,
      args.totalBets,
      args.currentLamports,
      args.activeVrfResults,
      args.referral,
      args.username
    )
  }

  /**
   * Deserializes the {@link UserAccount} from the data of the provided {@link web3.AccountInfo}.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static fromAccountInfo(
    accountInfo: web3.AccountInfo<Buffer>,
    offset = 0
  ): [UserAccount, number] {
    return UserAccount.deserialize(accountInfo.data, offset)
  }

  /**
   * Retrieves the account info from the provided address and deserializes
   * the {@link UserAccount} from its data.
   *
   * @throws Error if no account info is found at the address or if deserialization fails
   */
  static async fromAccountAddress(
    connection: web3.Connection,
    address: web3.PublicKey
  ): Promise<UserAccount> {
    const accountInfo = await connection.getAccountInfo(address)
    if (accountInfo == null) {
      throw new Error(`Unable to find UserAccount account at ${address}`)
    }
    return UserAccount.fromAccountInfo(accountInfo, 0)[0]
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
    return beetSolana.GpaBuilder.fromStruct(programId, userAccountBeet)
  }

  /**
   * Deserializes the {@link UserAccount} from the provided data Buffer.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static deserialize(buf: Buffer, offset = 0): [UserAccount, number] {
    return userAccountBeet.deserialize(buf, offset)
  }

  /**
   * Serializes the {@link UserAccount} into a Buffer.
   * @returns a tuple of the created Buffer and the offset up to which the buffer was written to store it.
   */
  serialize(): [Buffer, number] {
    return userAccountBeet.serialize(this)
  }

  /**
   * Returns the byteSize of a {@link Buffer} holding the serialized data of
   * {@link UserAccount} for the provided args.
   *
   * @param args need to be provided since the byte size for this account
   * depends on them
   */
  static byteSize(args: UserAccountArgs) {
    const instance = UserAccount.fromArgs(args)
    return userAccountBeet.toFixedFromValue(instance).byteSize
  }

  /**
   * Fetches the minimum balance needed to exempt an account holding
   * {@link UserAccount} data from rent
   *
   * @param args need to be provided since the byte size for this account
   * depends on them
   * @param connection used to retrieve the rent exemption information
   */
  static async getMinimumBalanceForRentExemption(
    args: UserAccountArgs,
    connection: web3.Connection,
    commitment?: web3.Commitment
  ): Promise<number> {
    return connection.getMinimumBalanceForRentExemption(
      UserAccount.byteSize(args),
      commitment
    )
  }

  /**
   * Returns a readable version of {@link UserAccount} properties
   * and can be used to convert to JSON and/or logging
   */
  pretty() {
    return {
      accountType: 'StateAccountType.' + StateAccountType[this.accountType],
      authority: this.authority.toBase58(),
      totalBets: this.totalBets,
      currentLamports: (() => {
        const x = <{ toNumber: () => number }>this.currentLamports
        if (typeof x.toNumber === 'function') {
          try {
            return x.toNumber()
          } catch (_) {
            return x
          }
        }
        return x
      })(),
      activeVrfResults: this.activeVrfResults,
      referral: this.referral,
      username: this.username,
    }
  }
}

/**
 * @category Accounts
 * @category generated
 */
export const userAccountBeet = new beet.FixableBeetStruct<
  UserAccount,
  UserAccountArgs
>(
  [
    ['accountType', stateAccountTypeBeet],
    ['authority', beetSolana.publicKey],
    ['totalBets', beet.u32],
    ['currentLamports', beet.u64],
    ['activeVrfResults', beet.u32],
    ['referral', beet.coption(beetSolana.publicKey)],
    ['username', beet.coption(beet.utf8String)],
  ],
  UserAccount.fromArgs,
  'UserAccount'
)