<script setup>
import { useFileStore } from '../stores/fileStore'
import { computed } from 'vue'

const store = useFileStore()

const downloads = computed(() => {
  const items = []
  store.downloadProgress.forEach((progress, filename) => {
    items.push({
      filename,
      ...progress
    })
  })
  return items
})

function formatBytes(bytes) {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}
</script>

<template>
  <div v-if="downloads.length > 0" class="download-progress-container">
    <div class="download-header">
      <h3>正在下载 ({{ downloads.length }})</h3>
    </div>
    <div class="download-list">
      <div
        v-for="download in downloads"
        :key="download.filename"
        class="download-item"
      >
        <div class="download-info">
          <div class="filename">{{ download.filename }}</div>
          <div class="progress-text">
            {{ formatBytes(download.received) }} / {{ formatBytes(download.total) }}
            ({{ download.percentage }}%)
          </div>
        </div>
        <div class="progress-bar">
          <div
            class="progress-fill"
            :style="{ width: download.percentage + '%' }"
          ></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-progress-container {
  position: fixed;
  bottom: 20px;
  right: 20px;
  width: 350px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  overflow: hidden;
  z-index: 1000;
}

.download-header {
  padding: 12px 16px;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
}

.download-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.download-list {
  max-height: 300px;
  overflow-y: auto;
}

.download-item {
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
}

.download-item:last-child {
  border-bottom: none;
}

.download-info {
  margin-bottom: 8px;
}

.filename {
  font-size: 13px;
  font-weight: 500;
  color: #333;
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.progress-text {
  font-size: 11px;
  color: #666;
}

.progress-bar {
  height: 4px;
  background: #e0e0e0;
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4CAF50, #45a049);
  transition: width 0.3s ease;
}
</style>
