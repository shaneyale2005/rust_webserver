<script setup>
import { onMounted, onUnmounted } from 'vue'
import { useFileStore } from '../stores/fileStore'
import Breadcrumb from './Breadcrumb.vue'
import Toolbar from './Toolbar.vue'
import FileList from './FileList.vue'
import FileGrid from './FileGrid.vue'
import PreviewModal from './PreviewModal.vue'
import DownloadProgress from './DownloadProgress.vue'
import { ArrowLeftIcon, HomeIcon } from '../assets/icons'

const store = useFileStore()

function handleKeydown(e) {
  const maxIndex = store.sortedFiles.length - 1

  if (store.sortedFiles.length === 0) return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    store.setKeyboardIndex(
      store.keyboardIndex < maxIndex ? store.keyboardIndex + 1 : 0
    )
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    store.setKeyboardIndex(
      store.keyboardIndex > 0 ? store.keyboardIndex - 1 : maxIndex
    )
  } else if (e.key === 'ArrowLeft') {
    if (store.currentPath !== '/') {
      e.preventDefault()
      store.navigateUp()
    }
  } else if (e.key === 'Home' || e.key === 'h') {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault()
      store.goHome()
    }
  }
}

onMounted(() => {
  store.loadDirectory('/')
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="file-manager">
    <header class="header">
      <div class="header-top">
        <h1 class="title">File Browser</h1>
        <div class="header-actions">
          <button
            v-if="store.currentPath !== '/'"
            class="nav-btn"
            @click="store.navigateUp()"
            title="Go back"
          >
            <ArrowLeftSolid />
            <span>Back</span>
          </button>
          <button
            class="nav-btn"
            @click="store.goHome()"
            title="Go home"
          >
            <HomeSolid />
            <span>Home</span>
          </button>
        </div>
      </div>
      <Breadcrumb />
    </header>

    <div class="main-content">
      <Toolbar />

      <div class="content-area">
        <div v-if="store.error" class="error-message">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>{{ store.error }}</span>
        </div>

        <div v-if="store.loading" class="loading-state">
          <div class="spinner"></div>
          <span>Loading...</span>
        </div>

        <FileList v-else-if="store.viewMode === 'list'" />
        <FileGrid v-else-if="store.viewMode === 'grid'" />
      </div>
    </div>

    <footer class="footer">
      Powered by Rust Webserver
    </footer>

    <PreviewModal />
    <DownloadProgress />
  </div>
</template>

<style scoped>
.file-manager {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--color-cream, #FAF7F0);
  padding: 1.5rem;
}

.header {
  margin-bottom: 1.5rem;
}

.header-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.title {
  font-size: 1.75rem;
  font-weight: 600;
  color: var(--color-darker-brown, #4A3F35);
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
}

.nav-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 8px;
  background: white;
  color: var(--color-dark-brown, #6B5D52);
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.nav-btn:hover {
  background: var(--color-hover, #E8DCC4);
  border-color: var(--color-primary, #9C8671);
}

.nav-btn svg {
  width: 18px;
  height: 18px;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.content-area {
  flex: 1;
}

.error-message {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  background: #FEF2F2;
  border: 1px solid #FECACA;
  border-radius: 12px;
  color: #DC2626;
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem;
  color: var(--color-brown, #9C8671);
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--color-tan, #D4C5A9);
  border-top-color: var(--color-primary, #9C8671);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.footer {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-tan, #D4C5A9);
  text-align: center;
  font-size: 0.8125rem;
  color: var(--color-brown, #9C8671);
}

@media (max-width: 640px) {
  .file-manager {
    padding: 1rem;
  }

  .title {
    font-size: 1.375rem;
  }

  .nav-btn span {
    display: none;
  }

  .nav-btn {
    padding: 0.5rem;
  }
}
</style>
