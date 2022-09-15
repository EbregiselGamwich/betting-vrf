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
 * @category VrfResultMarkClose
 * @category generated
 */
export const VrfResultMarkCloseStruct = new beet.BeetArgsStruct<{
  instructionDiscriminator: number
}>([['instructionDiscriminator', beet.u8]], 'VrfResultMarkCloseInstructionArgs')
/**
 * Accounts required by the _VrfResultMarkClose_ instruction
 *
 * @property [**signer**] bettor Bettor wallet account
 * @property [_writable_] vrfResultPda VRF result PDA account
 * @category Instructions
 * @category VrfResultMarkClose
 * @category generated
 */
export type VrfResultMarkCloseInstructionAccounts = {
  bettor: web3.PublicKey
  vrfResultPda: web3.PublicKey
}

export const vrfResultMarkCloseInstructionDiscriminator = 11

/**
 * Creates a _VrfResultMarkClose_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @category Instructions
 * @category VrfResultMarkClose
 * @category generated
 */
export function createVrfResultMarkCloseInstruction(
  accounts: VrfResultMarkCloseInstructionAccounts,
  programId = new web3.PublicKey('HiEuiREGdSuBYv4oxtdkWnYtcnNUKk8m93XSn8pPYtcm')
) {
  const [data] = VrfResultMarkCloseStruct.serialize({
    instructionDiscriminator: vrfResultMarkCloseInstructionDiscriminator,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.bettor,
      isWritable: false,
      isSigner: true,
    },
    {
      pubkey: accounts.vrfResultPda,
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
