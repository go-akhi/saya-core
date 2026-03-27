<script setup lang="ts">
import { onMounted } from "vue";
import { useUserAccountsStore } from "../../stores/userAccounts";

const store = useUserAccountsStore();

function providerIcon(provider: string): string {
  switch (provider) {
    case "gmail": return "\u{1F4E7}";
    case "outlook": return "\u{1F4E8}";
    default: return "\u{1F464}";
  }
}

async function toggle(id: number) {
  await store.toggleActive(id);
}

async function remove(id: number) {
  if (!confirm("Remove this account and all associated data?")) return;
  await store.removeAccount(id);
}

onMounted(() => store.loadAccounts());
</script>

<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>Accounts</h3>
      <button class="btn-add" title="Add account (OAuth not yet configured)">+ Add Account</button>
    </div>

    <p v-if="store.isLoading" class="status-text">Loading...</p>
    <p v-else-if="store.accounts.length === 0" class="status-text">
      No accounts connected. Add an account to sync email and calendar data.
    </p>

    <div class="account-list">
      <div v-for="account in store.accounts" :key="account.id" class="account-card">
        <span class="account-icon">{{ providerIcon(account.provider) }}</span>
        <div class="account-info">
          <div class="account-email">{{ account.email }}</div>
          <div class="account-provider">
            {{ account.provider }}
            <span :class="['status-dot', account.is_active ? 'active' : 'inactive']" />
            {{ account.is_active ? "Active" : "Inactive" }}
          </div>
        </div>
        <div class="account-actions">
          <label class="toggle-switch" :title="account.is_active ? 'Disable' : 'Enable'">
            <input type="checkbox" :checked="account.is_active" @change="toggle(account.id)" />
            <span class="toggle-track" />
          </label>
          <button class="btn-icon-sm btn-danger" title="Remove" @click="remove(account.id)">&times;</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tab-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.tab-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.status-text {
  color: var(--text-muted);
  font-size: 13px;
}

.btn-add {
  padding: 4px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--accent);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 150ms;
}

.btn-add:hover {
  background-color: var(--bg-hover);
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.account-card {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  gap: 10px;
}

.account-icon {
  font-size: 20px;
  flex-shrink: 0;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-email {
  font-size: 13px;
  font-weight: 500;
}

.account-provider {
  font-size: 12px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.status-dot.active {
  background-color: #16a34a;
}

.status-dot.inactive {
  background-color: #d97706;
}

.account-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-switch {
  position: relative;
  cursor: pointer;
}

.toggle-switch input {
  display: none;
}

.toggle-track {
  display: block;
  width: 32px;
  height: 18px;
  border-radius: 9px;
  background-color: var(--bg-badge);
  transition: background-color 150ms;
  position: relative;
}

.toggle-track::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: white;
  box-shadow: 0 1px 2px rgba(0,0,0,0.15);
  transition: transform 150ms;
}

.toggle-switch input:checked + .toggle-track {
  background-color: var(--accent);
}

.toggle-switch input:checked + .toggle-track::after {
  transform: translateX(14px);
}

.btn-icon-sm {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
}

.btn-icon-sm:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.btn-icon-sm.btn-danger:hover {
  background-color: #fee2e2;
  color: #dc2626;
}
</style>
