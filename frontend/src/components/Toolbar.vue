<script setup>
import { computed, ref, watch } from 'vue'
import { useFileStore } from '../stores/fileStore'
import {
  MagnifyingGlassIcon,
  Squares2X2Icon,
  ListBulletIcon,
  ArrowPathIcon,
  CheckCircleIcon,
  XCircleIcon,
  ArrowPathRoundedSquareIcon,
  ArrowDownTrayIcon
} from '../assets/icons'

const store = useFileStore()
const searchInput = ref('')

const debouncedQuery = ref('')
let debounceTimer = null

watch(searchInput, (val) => {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debouncedQuery.value = val
    store.setSearchQuery(val)
  }, 300)
})

function toggleView() {
  store.setViewMode(store.viewMode === 'list' ? 'grid' : 'list')
}

function handleSelectAll() {
  if (store.allSelected) {
    store.deselectAll()
  } else {
    store.selectAll()
  }
}

function handleInvert() {
  store.invertSelection()
}

function handleDownload() {
  store.downloadSelected()
}

function refresh() {
  store.loadDirectory(store.currentPath)
}
</script>

<template>
  <div class="toolbar">
    <div class="search-box">
      <MagnifyingGlassSolid class="search-icon" />
      <input
        v-model="searchInput"
        type="text"
        placeholder="Search files..."
        class="search-input"
      />
    </div>

    <div class="toolbar-actions">
      <button class="toolbar-btn" @click="refresh" title="Refresh">
        <ArrowPathSolid />
      </button>

      <div class="view-toggle">
        <button
          class="view-btn"
          :class="{ active: store.viewMode === 'list' }"
          @click="store.viewMode === 'grid' && toggleView()"
          title="List view"
        >
          <ListBulletSolid />
        </button>
        <button
          class="view-btn"
          :class="{ active: store.viewMode === 'grid' }"
          @click="store.viewMode === 'list' && toggleView()"
          title="Grid view"
        >
          <Squares2X2Solid />
        </button>
      </div>

      <div class="divider"></div>

      <button class="toolbar-btn" @click="handleSelectAll" :title="store.allSelected ? 'Deselect all' : 'Select all'">
        <CheckCircleSolid v-if="store.allSelected" />
        <XCircleSolid v-else />
        <span class="btn-text">{{ store.allSelected ? 'Deselect all' : 'Select all' }}</span>
      </button>

      <button class="toolbar-btn" @click="handleInvert" title="Invert selection">
        <ArrowPathIconSolid />
        <span class="btn-text">Invert</span>
      </button>

      <button
        class="toolbar-btn primary"
        :disabled="store.selectedCount === 0"
        @click="handleDownload"
        title="Download selected"
      >
        <ArrowDownTraySolid />
        <span class="btn-text">Download ({{ store.selectedCount }})</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-tan, #D4C5A9);
  flex-wrap: wrap;
}

.search-box {
  position: relative;
  flex: 1;
  max-width: 320px;
  min-width: 200px;
}

.search-icon {
  position: absolute;
  left: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  width: 18px;
  height: 18px;
  color: var(--color-brown, #9C8671);
}

.search-input {
  width: 100%;
  padding: 0.5rem 0.75rem 0.5rem 2.5rem;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 8px;
  font-size: 0.875rem;
  background: white;
  color: var(--color-darker-brown, #4A3F35);
  transition: all 0.2s ease;
}

.search-input:focus {
  outline: none;
  border-color: var(--color-primary, #9C8671);
  box-shadow: 0 0 0 3px rgba(156, 134, 113, 0.15);
}

.search-input::placeholder {
  color: var(--color-brown, #9C8671);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.toolbar-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 8px;
  background: white;
  color: var(--color-dark-brown, #6B5D52);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.toolbar-btn:hover:not(:disabled) {
  background: var(--color-hover, #E8DCC4);
  border-color: var(--color-primary, #9C8671);
}

.toolbar-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toolbar-btn.primary {
  background: var(--color-primary, #9C8671);
  border-color: var(--color-primary, #9C8671);
  color: white;
}

.toolbar-btn.primary:hover:not(:disabled) {
  background: var(--color-dark-brown, #6B5D52);
  border-color: var(--color-dark-brown, #6B5D52);
}

.toolbar-btn svg {
  width: 18px;
  height: 18px;
}

.btn-text {
  display: none;
}

@media (min-width: 640px) {
  .btn-text {
    display: inline;
  }
}

.view-toggle {
  display: flex;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 8px;
  overflow: hidden;
}

.view-btn {
  padding: 0.5rem 0.625rem;
  background: white;
  border: none;
  color: var(--color-brown, #9C8671);
  cursor: pointer;
  transition: all 0.2s ease;
}

.view-btn:first-child {
  border-right: 1px solid var(--color-tan, #D4C5A9);
}

.view-btn:hover {
  background: var(--color-hover, #E8DCC4);
}

.view-btn.active {
  background: var(--color-primary, #9C8671);
  color: white;
}

.view-btn svg {
  width: 18px;
  height: 18px;
}

.divider {
  width: 1px;
  height: 24px;
  background: var(--color-tan, #D4C5A9);
  margin: 0 0.25rem;
}
</style>
