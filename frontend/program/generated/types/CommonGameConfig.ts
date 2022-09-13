/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
export type CommonGameConfig = {
  minWager: beet.bignum
  maxWager: beet.bignum
}

/**
 * @category userTypes
 * @category generated
 */
export const commonGameConfigBeet = new beet.BeetArgsStruct<CommonGameConfig>(
  [
    ['minWager', beet.u64],
    ['maxWager', beet.u64],
  ],
  'CommonGameConfig'
)