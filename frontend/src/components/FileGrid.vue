<script setup>
import { useFileStore } from '../stores/fileStore'
import FileItem from './FileItem.vue'
import Checkbox from './Checkbox.vue'

const store = useFileStore()

function handleToggle(file) {
  store.toggleSelect(file.name)
}

function handleOpen(file) {
  store.openFile(file)
}

function handlePreview(file) {
  store.previewTextFile(file)
}
</script>

<template>
  <div class="file-grid">
    <div v-if="store.sortedFiles.length === 0" class="empty-state">
      <p>This directory is empty</p>
    </div>

    <div
      v-for="(file, index) in store.sortedFiles"
      :key="file.name"
      class="grid-item-wrapper"
      :class="{ active: store.keyboardIndex === index }"
    >
      <FileItem
        :file="file"
        :view-mode="'grid'"
        :selected="store.isSelected(file.name)"
        :active="store.keyboardIndex === index"
        @toggle="handleToggle"
        @open="handleOpen"
        @preview="handlePreview"
        tabindex="0"
      >
        <template #checkbox>
          <Checkbox
            :model-value="store.isSelected(file.name)"
            @update:model-value="handleToggle(file)"
          />
        </template>
      </FileItem>
    </div>
  </div>
</template>

<style scoped>
.file-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 0.75rem;
  padding: 0.5rem;
}

.empty-state {
  grid-column: 1 / -1;
  padding: 3rem 1rem;
  text-align: center;
  color: var(--color-brown, #9C8671);
}

.grid-item-wrapper {
  background: white;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 12px;
  overflow: hidden;
  transition: all 0.15s ease;
}

.grid-item-wrapper:hover {
  border-color: var(--color-primary, #9C8671);
  box-shadow: 0 4px 12px rgba(107, 93, 82, 0.1);
}

.grid-item-wrapper.active {
  border-color: var(--color-primary, #9C8671);
  box-shadow: 0 0 0 2px var(--color-primary, #9C8671);
}
</style>
