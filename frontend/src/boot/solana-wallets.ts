import {
  ExodusWalletAdapter,
  LedgerWalletAdapter,
  MathWalletAdapter,
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  SolletWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { Connection } from '@solana/web3.js';
import { boot } from 'quasar/wrappers';
import SolanaWallets from 'solana-wallets-vue';
import 'solana-wallets-vue/styles.css';

// "async" is optional;
// more info on params: https://v2.quasar.dev/quasar-cli/boot-files
export default boot(({ app }) => {
  const connection = new Connection('https://rpc.ankr.com/solana_devnet');
  const walletOptions = {
    wallets: [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new SolletWalletAdapter(),
      new ExodusWalletAdapter(),
      new MathWalletAdapter(),
      new LedgerWalletAdapter(),
    ],
    autoConnect: true,
  };
  app.provide('connection', connection);
  app.use(SolanaWallets, walletOptions);
});
