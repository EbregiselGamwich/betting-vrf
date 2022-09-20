<script lang="ts">
import { computed, defineComponent, ref, toRefs } from 'vue';
import { onClickOutside, useClipboard } from '@vueuse/core';
import { useWallet, WalletIcon, WalletModalProvider } from 'solana-wallets-vue';
import CustomWalletConnectButtonVue from './CustomWalletConnectButton.vue';
import { useQuasar } from 'quasar';
import { useI18n } from 'vue-i18n';

export default defineComponent({
  components: {
    WalletIcon,
    WalletModalProvider,
    CustomWalletConnectButtonVue,
  },
  props: {
    featured: { type: Number, default: 3 },
    container: { type: String, default: 'body' },
    logo: String,
    dark: Boolean,
  },
  setup(props) {
    const { featured, container, logo, dark } = toRefs(props);
    const { publicKey, wallet, disconnect } = useWallet();

    const dropdownPanel = ref<HTMLElement>();
    const dropdownOpened = ref(false);
    const openDropdown = () => (dropdownOpened.value = true);
    const closeDropdown = () => (dropdownOpened.value = false);
    onClickOutside(dropdownPanel, closeDropdown);

    const publicKeyBase58 = computed(() => publicKey.value?.toBase58());
    const publicKeyTrimmed = computed(() => {
      if (!wallet.value || !publicKeyBase58.value) return null;
      return (
        publicKeyBase58.value.slice(0, 4) +
        '..' +
        publicKeyBase58.value.slice(-4)
      );
    });

    const {
      copy,
      copied: addressCopied,
      isSupported: canCopy,
    } = useClipboard();

    const $q = useQuasar();
    const { t: $t } = useI18n();
    const copyAddress = () => {
      publicKeyBase58.value && copy(publicKeyBase58.value);
      $q.notify({
        progress: true,
        message: $t('wallet.addressCopied'),
      });
    };
    // Define the bindings given to scoped slots.
    const scope = {
      featured,
      container,
      logo,
      dark,
      wallet,
      publicKey,
      publicKeyTrimmed,
      publicKeyBase58,
      canCopy,
      addressCopied,
      dropdownPanel,
      dropdownOpened,
      openDropdown,
      closeDropdown,
      copyAddress,
      disconnect,
    };

    return {
      scope,
      ...scope,
    };
  },
});
</script>

<template>
  <wallet-modal-provider
    :featured="featured"
    :container="container"
    :logo="logo"
    :dark="dark"
  >
    <template #default="modalScope">
      <slot v-bind="{ ...modalScope, ...scope }">
        <q-btn
          outline
          round
          icon="wallet"
          v-if="!wallet"
          @click="modalScope.openModal"
        ></q-btn>
        <custom-wallet-connect-button-vue
          v-else-if="!publicKeyBase58"
        ></custom-wallet-connect-button-vue>
        <div v-else>
          <slot name="dropdown-button" v-bind="{ ...modalScope, ...scope }">
            <q-btn outline round>
              <wallet-icon :wallet="wallet"></wallet-icon>
              <q-menu
                style="min-width: fit-content"
                v-bind="{ ...modalScope, ...scope, dark: false }"
              >
                <q-list>
                  <q-item>
                    <q-item-section>{{ publicKeyTrimmed }}</q-item-section>
                  </q-item>
                  <q-separator></q-separator>
                  <q-item
                    v-if="canCopy"
                    clickable
                    v-close-popup
                    @click="copyAddress"
                  >
                    {{
                      addressCopied
                        ? $t('wallet.addressCopied')
                        : $t('wallet.copyAddress')
                    }}
                  </q-item>
                  <q-item
                    clickable
                    v-close-popup
                    @click="modalScope.openModal()"
                  >
                    {{ $t('wallet.changeWallet') }}
                  </q-item>
                  <q-item clickable v-close-popup @click="disconnect">
                    {{ $t('wallet.disconnect') }}
                  </q-item>
                </q-list>
              </q-menu>
            </q-btn>
          </slot>
        </div>
      </slot>
    </template>

    <!-- Enable modal overrides. -->
    <template #overlay="modalScope">
      <slot name="modal-overlay" v-bind="{ ...modalScope, ...scope }"></slot>
    </template>
    <template #modal="modalScope">
      <slot name="modal" v-bind="{ ...modalScope, ...scope }"></slot>
    </template>
  </wallet-modal-provider>
</template>
