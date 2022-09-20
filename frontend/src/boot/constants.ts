import { PublicKey } from '@solana/web3.js';
import { boot } from 'quasar/wrappers';
import * as program from '../../program/generated';

// "async" is optional;
// more info on params: https://v2.quasar.dev/quasar-cli/boot-files
export default boot(async ({ app }) => {
  app.config.globalProperties.statsPDA = (
    await PublicKey.findProgramAddress(
      [Buffer.from('Stats', 'utf8')],
      program.PROGRAM_ID
    )
  )[0];
});
