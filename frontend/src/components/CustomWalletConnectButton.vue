<script lang="ts">
import { computed, defineComponent, toRefs } from 'vue';
import { useWallet, WalletIcon } from 'solana-wallets-vue';

export default defineComponent({
  components: {
    WalletIcon,
  },
  props: {
    disabled: Boolean,
  },
  setup(props, { emit }) {
    const { disabled } = toRefs(props);
    const { wallet, connect, connecting, connected } = useWallet();

    const content = computed(() => {
      if (connecting.value) return 'Connecting ...';
      if (connected.value) return 'Connected';
      if (wallet.value) return 'Connect';
      return 'Connect Wallet';
    });

    const onClick = (event: Event) => {
      emit('click', event);
      if (event.defaultPrevented) return;
      connect().catch((e) => {
        console.log(e);
      });
    };

    const scope = {
      wallet,
      disabled,
      connecting,
      connected,
      content,
      onClick,
    };

    return {
      scope,
      ...scope,
    };
  },
});
</script>

<template>
  <slot v-bind="scope">
    <q-btn
      outline
      round
      :loading="connecting"
      :disabled="disabled || !wallet || connecting || connected"
      @click="onClick"
    >
      <q-icon name="warning" v-if="content == 'Connect'"></q-icon>
      <wallet-icon v-else-if="wallet" :wallet="wallet"></wallet-icon>
    </q-btn>
  </slot>
</template>
