<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>Sync</h3>
    </div>
    <div class="placeholder">
      <div class="placeholder-icon">&#128260;</div>
      <p class="placeholder-title">Iroh Sync</p>
      <p class="placeholder-desc">
        Peer-to-peer sync between devices will be available in a future release.
        This includes device pairing, sync status, and conflict resolution.
      </p>
    </div>
  </div>
</template>

<style scoped>
.settings-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tab-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
}

.placeholder-icon {
  font-size: 32px;
  margin-bottom: 12px;
  opacity: 0.5;
}

.placeholder-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}

.placeholder-desc {
  font-size: 13px;
  color: var(--text-muted);
  max-width: 320px;
  line-height: 1.5;
}
</style>
