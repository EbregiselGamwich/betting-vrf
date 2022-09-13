/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'
import {
  GameSetActiveArgs,
  gameSetActiveArgsBeet,
} from '../types/GameSetActiveArgs'

/**
 * @category Instructions
 * @category GameSetActive
 * @category generated
 */
export type GameSetActiveInstructionArgs = {
  gameSetActiveArgs: GameSetActiveArgs
}
/**
 * @category Instructions
 * @category GameSetActive
 * @category generated
 */
export const GameSetActiveStruct = new beet.BeetArgsStruct<
  GameSetActiveInstructionArgs & {
    instructionDiscriminator: number
  }
>(
  [
    ['instructionDiscriminator', beet.u8],
    ['gameSetActiveArgs', gameSetActiveArgsBeet],
  ],
  'GameSetActiveInstructionArgs'
)
/**
 * Accounts required by the _GameSetActive_ instruction
 *
 * @property [_writable_, **signer**] host The wallet account of the host
 * @property [_writable_] gamePda Game PDA Account
 * @category Instructions
 * @category GameSetActive
 * @category generated
 */
export type GameSetActiveInstructionAccounts = {
  host: web3.PublicKey
  gamePda: web3.PublicKey
}

export const gameSetActiveInstructionDiscriminator = 6

/**
 * Creates a _GameSetActive_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category GameSetActive
 * @category generated
 */
export function createGameSetActiveInstruction(
  accounts: GameSetActiveInstructionAccounts,
  args: GameSetActiveInstructionArgs,
  programId = new web3.PublicKey('HiEuiREGdSuBYv4oxtdkWnYtcnNUKk8m93XSn8pPYtcm')
) {
  const [data] = GameSetActiveStruct.serialize({
    instructionDiscriminator: gameSetActiveInstructionDiscriminator,
    ...args,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.host,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: accounts.gamePda,
      isWritable: true,
      isSigner: false,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId,
    keys,
    data,
  })
  return ix
}
