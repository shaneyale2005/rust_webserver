<script setup>
import { computed } from 'vue'
import { useFileStore } from '../stores/fileStore'
import {
  FolderIcon,
  DocumentIcon,
  ChevronRightIcon
} from '../assets/icons'

const store = useFileStore()

const pathParts = computed(() => {
  const parts = store.currentPath.split('/').filter(Boolean)
  return parts.map((name, index) => ({
    name,
    path: '/' + parts.slice(0, index + 1).join('/')
  }))
})

function navigate(index) {
  store.navigateTo(index === -1 ? '/' : pathParts.value[index].path)
}
</script>

<template>
  <nav class="breadcrumb">
    <ul class="breadcrumb-list">
      <li class="breadcrumb-item">
        <button class="breadcrumb-link home-link" @click="navigate(-1)" title="Home">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 3L4 9v12h5v-7h6v7h5V9z"/>
          </svg>
        </button>
      </li>
      <li v-for="(part, index) in pathParts" :key="part.path" class="breadcrumb-item">
        <ChevronRightSolid class="separator" />
        <button
          class="breadcrumb-link"
          :class="{ current: index === pathParts.length - 1 }"
          @click="navigate(index)"
        >
          {{ part.name }}
        </button>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
.breadcrumb {
  padding: 0.75rem 0;
}

.breadcrumb-list {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.25rem;
  list-style: none;
  margin: 0;
  padding: 0;
}

.breadcrumb-item {
  display: flex;
  align-items: center;
}

.separator {
  width: 16px;
  height: 16px;
  color: var(--color-brown, #9C8671);
}

.breadcrumb-link {
  padding: 0.375rem 0.75rem;
  border-radius: 6px;
  font-size: 0.875rem;
  color: var(--color-dark-brown, #6B5D52);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
  font-family: inherit;
}

.breadcrumb-link:hover:not(.current) {
  background: var(--color-hover, #E8DCC4);
  color: var(--color-darker-brown, #4A3F35);
}

.breadcrumb-link.current {
  color: var(--color-darker-brown, #4A3F35);
  font-weight: 600;
  background: var(--color-sand, #E8DCC4);
}

.home-link {
  padding: 0.375rem;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-primary, #9C8671);
}

.home-link:hover {
  background: var(--color-hover, #E8DCC4);
  color: var(--color-dark-brown, #6B5D52);
}

.home-link svg {
  width: 18px;
  height: 18px;
}
</style>
