<script setup>
import { computed } from 'vue'
import { useFileStore } from '../stores/fileStore'
import { FolderIcon, DocumentIcon, DocumentTextIcon } from '../assets/icons'

const props = defineProps({
  file: {
    type: Object,
    required: true
  },
  viewMode: {
    type: String,
    default: 'list'
  },
  selected: {
    type: Boolean,
    default: false
  },
  active: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['toggle', 'open', 'preview', 'keydown'])

const store = useFileStore()

const isTextFile = computed(() => {
  const name = props.file.name.toLowerCase()
  return name.endsWith('.txt') ||
         name.endsWith('.md') ||
         name.endsWith('.json') ||
         name.endsWith('.js') ||
         name.endsWith('.css') ||
         name.endsWith('.html') ||
         name.endsWith('.xml') ||
         name.endsWith('.log')
})

function handleClick(e) {
  if (e.shiftKey || e.ctrlKey || e.metaKey) {
    emit('toggle', props.file)
  } else {
    emit('open', props.file)
  }
}

function handleDoubleClick() {
  emit('open', props.file)
}

function handleMiddleClick(e) {
  if (e.button === 1) {
    emit('open', props.file)
  }
}

function handlePreview(e) {
  if (props.file.type === 'file' && isTextFile.value) {
    e.stopPropagation()
    emit('preview', props.file)
  }
}
</script>

<template>
  <div
    class="file-item"
    :class="[
      viewMode,
      { selected, active }
    ]"
    @click="handleClick"
    @dblclick="handleDoubleClick"
    @mousedown.middle="handleMiddleClick"
    @keydown="$emit('keydown', $event)"
    tabindex="0"
  >
    <div class="file-checkbox" @click.stop="emit('toggle', file)">
      <slot name="checkbox" />
    </div>

    <div class="file-icon" @click="handlePreview">
      <FolderIcon v-if="file.type === 'dir'" class="icon folder" />
      <DocumentTextIcon v-else-if="isTextFile" class="icon text" />
      <DocumentIcon v-else class="icon file" />
    </div>

    <div class="file-info">
      <span class="file-name" :title="file.name">{{ file.name }}</span>
      <span v-if="viewMode === 'list'" class="file-meta">
        <span class="file-size">{{ file.size }}</span>
        <span class="file-date">{{ new Date(file.date).toLocaleString() }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.file-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.625rem 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  background: white;
}

.file-item:hover {
  background: var(--color-hover, #E8DCC4);
}

.file-item.selected {
  background: var(--color-tan, #D4C5A9);
}

.file-item.active {
  background: var(--color-tan, #D4C5A9);
  box-shadow: inset 0 0 0 2px var(--color-primary, #9C8671);
}

.file-checkbox {
  flex-shrink: 0;
}

.file-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
}

.file-icon .icon {
  width: 28px;
  height: 28px;
}

.file-icon .folder {
  color: var(--color-primary, #9C8671);
}

.file-icon .text {
  color: #5B8C5A;
}

.file-icon .file {
  color: var(--color-brown, #9C8671);
}

.file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.file-name {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-darker-brown, #4A3F35);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-meta {
  display: flex;
  gap: 1rem;
  font-size: 0.75rem;
  color: var(--color-brown, #9C8671);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

/* Grid mode */
.file-item.grid {
  flex-direction: column;
  padding: 1rem;
  text-align: center;
  gap: 0.5rem;
}

.file-item.grid .file-icon {
  width: 48px;
  height: 48px;
}

.file-item.grid .file-icon .icon {
  width: 40px;
  height: 40px;
}

.file-item.grid .file-checkbox {
  position: absolute;
  top: 0.5rem;
  left: 0.5rem;
  z-index: 10;
}

.file-item.grid {
  position: relative;
}

.file-item.grid .file-info {
  align-items: center;
}

.file-item.grid .file-name {
  max-width: 100%;
  font-size: 0.8125rem;
}

.file-item.grid .file-meta {
  display: none;
}
</style>
