/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import { CoinFlipConfig, coinFlipConfigBeet } from './CoinFlipConfig';
import { CrashConfig, crashConfigBeet } from './CrashConfig';
/**
 * This type is used to derive the {@link GameTypeConfig} type as well as the de/serializer.
 * However don't refer to it in your code but use the {@link GameTypeConfig} type instead.
 *
 * @category userTypes
 * @category enums
 * @category generated
 * @private
 */
export type GameTypeConfigRecord = {
  CoinFlip: { config: CoinFlipConfig };
  Crash: { config: CrashConfig };
};

/**
 * Union type respresenting the GameTypeConfig data enum defined in Rust.
 *
 * NOTE: that it includes a `__kind` property which allows to narrow types in
 * switch/if statements.
 * Additionally `isGameTypeConfig*` type guards are exposed below to narrow to a specific variant.
 *
 * @category userTypes
 * @category enums
 * @category generated
 */
export type GameTypeConfig = beet.DataEnumKeyAsKind<GameTypeConfigRecord>;

export const isGameTypeConfigCoinFlip = (
  x: GameTypeConfig
): x is GameTypeConfig & { __kind: 'CoinFlip' } => x.__kind === 'CoinFlip';
export const isGameTypeConfigCrash = (
  x: GameTypeConfig
): x is GameTypeConfig & { __kind: 'Crash' } => x.__kind === 'Crash';

/**
 * @category userTypes
 * @category generated
 */
export const gameTypeConfigBeet = beet.dataEnum<GameTypeConfigRecord>([
  [
    'CoinFlip',
    new beet.BeetArgsStruct<GameTypeConfigRecord['CoinFlip']>(
      [['config', coinFlipConfigBeet]],
      'GameTypeConfigRecord["CoinFlip"]'
    ),
  ],

  [
    'Crash',
    new beet.BeetArgsStruct<GameTypeConfigRecord['Crash']>(
      [['config', crashConfigBeet]],
      'GameTypeConfigRecord["Crash"]'
    ),
  ],
]) as beet.FixableBeet<GameTypeConfig>;
