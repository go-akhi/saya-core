import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface UserAccount {
  id: number;
  provider: string;
  email: string;
  is_active: boolean;
  created_at: string | null;
}

export const useUserAccountsStore = defineStore("userAccounts", () => {
  const accounts = ref<UserAccount[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function loadAccounts() {
    isLoading.value = true;
    error.value = null;
    try {
      accounts.value = await invoke<UserAccount[]>("get_user_accounts");
    } catch (e) {
      error.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function toggleActive(id: number) {
    try {
      const newState = await invoke<boolean>("toggle_account_active", { id });
      const idx = accounts.value.findIndex((a) => a.id === id);
      if (idx !== -1) {
        accounts.value[idx].is_active = newState;
      }
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function removeAccount(id: number) {
    try {
      await invoke("delete_user_account", { id });
      accounts.value = accounts.value.filter((a) => a.id !== id);
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  return {
    accounts,
    isLoading,
    error,
    loadAccounts,
    toggleActive,
    removeAccount,
  };
});
