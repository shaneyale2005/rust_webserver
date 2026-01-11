<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useFileStore } from '../stores/fileStore'
import { XMarkIcon } from '../assets/icons'

const store = useFileStore()
const modalRef = ref(null)
const contentRef = ref(null)

function close() {
  store.closePreview()
}

function handleBackdrop(e) {
  if (e.target === e.currentTarget) {
    close()
  }
}

function handleKeydown(e) {
  if (e.key === 'Escape') {
    close()
  }
}

function copyContent() {
  if (contentRef.value) {
    navigator.clipboard.writeText(contentRef.value.textContent)
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

watch(() => store.previewFile, (val) => {
  if (val) {
    document.body.style.overflow = 'hidden'
  } else {
    document.body.style.overflow = ''
  }
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="store.previewFile" class="modal-backdrop" @click="handleBackdrop">
        <div class="modal-content" ref="modalRef">
          <div class="modal-header">
            <div class="file-info">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/>
                <path d="M14 2v6h6"/>
                <path d="M16 13H8"/>
                <path d="M16 17H8"/>
                <path d="M10 9H8"/>
              </svg>
              <span class="file-name">{{ store.previewFile.name }}</span>
              <span class="file-size">({{ store.previewFile.size }})</span>
            </div>
            <div class="modal-actions">
              <button class="action-btn" @click="copyContent" title="Copy content">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                </svg>
                Copy
              </button>
              <button class="close-btn" @click="close" title="Close (Esc)">
                <XMarkSolid />
              </button>
            </div>
          </div>
          <div class="modal-body">
            <pre ref="contentRef" class="file-content">{{ store.previewContent }}</pre>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(74, 63, 53, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.modal-content {
  background: white;
  border-radius: 16px;
  width: 100%;
  max-width: 800px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid var(--color-tan, #D4C5A9);
  background: var(--color-sand, #E8DCC4);
}

.file-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--color-dark-brown, #6B5D52);
}

.file-info svg {
  color: var(--color-primary, #9C8671);
}

.file-name {
  font-weight: 600;
  color: var(--color-darker-brown, #4A3F35);
}

.file-size {
  font-size: 0.8125rem;
  color: var(--color-brown, #9C8671);
}

.modal-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 6px;
  background: white;
  color: var(--color-dark-brown, #6B5D52);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.action-btn:hover {
  background: var(--color-hover, #E8DCC4);
}

.close-btn {
  padding: 0.5rem;
  border: none;
  background: transparent;
  color: var(--color-brown, #9C8671);
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.2s ease;
}

.close-btn:hover {
  background: var(--color-hover, #E8DCC4);
  color: var(--color-darker-brown, #4A3F35);
}

.close-btn svg {
  width: 22px;
  height: 22px;
}

.modal-body {
  flex: 1;
  overflow: auto;
  padding: 1rem;
  background: #FAFAFA;
}

.file-content {
  margin: 0;
  padding: 1rem;
  background: white;
  border: 1px solid var(--color-tan, #D4C5A9);
  border-radius: 8px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8125rem;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-darker-brown, #4A3F35);
  max-height: 60vh;
  overflow: auto;
}

/* Transitions */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.3s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-content,
.modal-leave-to .modal-content {
  transform: scale(0.95) translateY(10px);
}
</style>
