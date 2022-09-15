/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'

/**
 * @category Instructions
 * @category VrfResultClose
 * @category generated
 */
export const VrfResultCloseStruct = new beet.BeetArgsStruct<{
  instructionDiscriminator: number
}>([['instructionDiscriminator', beet.u8]], 'VrfResultCloseInstructionArgs')
/**
 * Accounts required by the _VrfResultClose_ instruction
 *
 * @property [_writable_] vrfResultPda VRF result PDA account
 * @property [_writable_, **signer**] bettor Bettor wallet account
 * @property [_writable_] bettorUserAccount Bettor user account
 * @category Instructions
 * @category VrfResultClose
 * @category generated
 */
export type VrfResultCloseInstructionAccounts = {
  vrfResultPda: web3.PublicKey
  bettor: web3.PublicKey
  bettorUserAccount: web3.PublicKey
  systemProgram?: web3.PublicKey
}

export const vrfResultCloseInstructionDiscriminator = 11

/**
 * Creates a _VrfResultClose_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @category Instructions
 * @category VrfResultClose
 * @category generated
 */
export function createVrfResultCloseInstruction(
  accounts: VrfResultCloseInstructionAccounts,
  programId = new web3.PublicKey('HiEuiREGdSuBYv4oxtdkWnYtcnNUKk8m93XSn8pPYtcm')
) {
  const [data] = VrfResultCloseStruct.serialize({
    instructionDiscriminator: vrfResultCloseInstructionDiscriminator,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.vrfResultPda,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.bettor,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: accounts.bettorUserAccount,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.systemProgram ?? web3.SystemProgram.programId,
      isWritable: false,
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