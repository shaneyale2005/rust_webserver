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

function handleKeyDown(e, index) {
  if (e.key === 'Enter') {
    e.preventDefault()
    handleOpen(store.sortedFiles[index])
  } else if (e.key === ' ') {
    e.preventDefault()
    handleToggle(store.sortedFiles[index])
  }
}
</script>

<template>
  <div class="file-list">
    <div class="list-header">
      <div class="col-checkbox">
        <Checkbox
          :model-value="store.allSelected"
          :indeterminate="store.someSelected"
          @update:model-value="store.allSelected ? store.deselectAll() : store.selectAll()"
        />
      </div>
      <div class="col-name">Name</div>
      <div class="col-size">Size</div>
      <div class="col-date">Modified</div>
    </div>

    <div v-if="store.sortedFiles.length === 0" class="empty-state">
      <p>This directory is empty</p>
    </div>

    <div
      v-for="(file, index) in store.sortedFiles"
      :key="file.name"
      class="list-row-wrapper"
      :class="{ active: store.keyboardIndex === index }"
    >
      <FileItem
        :file="file"
        :view-mode="'list'"
        :selected="store.isSelected(file.name)"
        :active="store.keyboardIndex === index"
        @toggle="handleToggle"
        @open="handleOpen"
        @preview="handlePreview"
        @keydown="handleKeyDown($event, index)"
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
.file-list {
  background: white;
  border-radius: 12px;
  border: 1px solid var(--color-tan, #D4C5A9);
  overflow: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--color-sand, #E8DCC4);
  border-bottom: 1px solid var(--color-tan, #D4C5A9);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-dark-brown, #6B5D52);
}

.col-checkbox {
  width: 20px;
  flex-shrink: 0;
}

.col-name {
  flex: 1;
  min-width: 0;
}

.col-size {
  width: 80px;
  flex-shrink: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.col-date {
  width: 160px;
  flex-shrink: 0;
  text-align: right;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.empty-state {
  padding: 3rem 1rem;
  text-align: center;
  color: var(--color-brown, #9C8671);
}

.list-row-wrapper {
  border-bottom: 1px solid var(--color-sand, #E8DCC4);
}

.list-row-wrapper:last-child {
  border-bottom: none;
}

.list-row-wrapper.active {
  background: var(--color-tan, #D4C5A9);
}
</style>
