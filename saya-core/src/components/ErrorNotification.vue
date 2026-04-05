<script setup lang="ts">
import { useUiStore } from "../stores/ui";

const uiStore = useUiStore();
</script>

<template>
  <div class="error-container">
    <div
      v-for="notification in uiStore.notifications"
      :key="notification.id"
      class="error-banner"
      :class="notification.type"
    >
      <div class="error-content">
        <span class="error-title" v-if="notification.title">{{ notification.title }}: </span>
        <span class="error-message">{{ notification.message }}</span>
        <span class="error-source" v-if="notification.pluginName"> ({{ notification.pluginName }})</span>
      </div>
      <button class="dismiss-btn" @click="uiStore.dismissError(notification.id)">&times;</button>
    </div>
  </div>
</template>

<style scoped>
.error-container {
  position: fixed;
  top: 60px;
  right: 16px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 400px;
}

.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-radius: var(--radius);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  animation: slideIn 0.2s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.error-banner.error {
  background-color: #fef2f2;
  border: 1px solid #fecaca;
  color: #991b1b;
}

.error-banner.warning {
  background-color: #fffbeb;
  border: 1px solid #fde68a;
  color: #92400e;
}

.error-banner.info {
  background-color: #eff6ff;
  border: 1px solid #bfdbfe;
  color: #1e40af;
}

.error-content {
  flex: 1;
  font-size: 13px;
  line-height: 1.4;
}

.error-title {
  font-weight: 600;
}

.error-source {
  font-size: 12px;
  opacity: 0.8;
}

.dismiss-btn {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
  opacity: 0.6;
  color: inherit;
}

.dismiss-btn:hover {
  opacity: 1;
}
</style>
