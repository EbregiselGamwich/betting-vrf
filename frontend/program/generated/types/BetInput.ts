/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import { CoinFlipInput, coinFlipInputBeet } from './CoinFlipInput';
import { CrashInput, crashInputBeet } from './CrashInput';
/**
 * This type is used to derive the {@link BetInput} type as well as the de/serializer.
 * However don't refer to it in your code but use the {@link BetInput} type instead.
 *
 * @category userTypes
 * @category enums
 * @category generated
 * @private
 */
export type BetInputRecord = {
  CoinFlip: { input: CoinFlipInput };
  Crash: { input: CrashInput };
};

/**
 * Union type respresenting the BetInput data enum defined in Rust.
 *
 * NOTE: that it includes a `__kind` property which allows to narrow types in
 * switch/if statements.
 * Additionally `isBetInput*` type guards are exposed below to narrow to a specific variant.
 *
 * @category userTypes
 * @category enums
 * @category generated
 */
export type BetInput = beet.DataEnumKeyAsKind<BetInputRecord>;

export const isBetInputCoinFlip = (
  x: BetInput
): x is BetInput & { __kind: 'CoinFlip' } => x.__kind === 'CoinFlip';
export const isBetInputCrash = (
  x: BetInput
): x is BetInput & { __kind: 'Crash' } => x.__kind === 'Crash';

/**
 * @category userTypes
 * @category generated
 */
export const betInputBeet = beet.dataEnum<BetInputRecord>([
  [
    'CoinFlip',
    new beet.BeetArgsStruct<BetInputRecord['CoinFlip']>(
      [['input', coinFlipInputBeet]],
      'BetInputRecord["CoinFlip"]'
    ),
  ],

  [
    'Crash',
    new beet.BeetArgsStruct<BetInputRecord['Crash']>(
      [['input', crashInputBeet]],
      'BetInputRecord["Crash"]'
    ),
  ],
]) as beet.FixableBeet<BetInput>;
