import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { fetchFiles, fetchFileContent } from '../api/file'

export const useFileStore = defineStore('file', () => {
  const files = ref([])
  const currentPath = ref('/')
  const loading = ref(false)
  const error = ref(null)
  const selectedIds = ref(new Set())
  const viewMode = ref('list')
  const searchQuery = ref('')
  const previewContent = ref(null)
  const previewFile = ref(null)
  const keyboardIndex = ref(-1)

  const filteredFiles = computed(() => {
    if (!searchQuery.value.trim()) {
      return files.value
    }
    const query = searchQuery.value.toLowerCase()
    return files.value.filter(f => f.name.toLowerCase().includes(query))
  })

  const sortedFiles = computed(() => {
    return [...filteredFiles.value].sort((a, b) => {
      if (a.type === b.type) return a.name.localeCompare(b.name)
      return a.type === 'dir' ? -1 : 1
    })
  })

  const selectedCount = computed(() => selectedIds.value.size)
  const totalCount = computed(() => files.value.length)
  const allSelected = computed(() => totalCount.value > 0 && selectedCount.value === totalCount.value)
  const someSelected = computed(() => selectedCount.value > 0 && !allSelected.value)

  async function loadDirectory(path) {
    loading.value = true
    error.value = null
    keyboardIndex.value = -1
    selectedIds.value.clear()

    try {
      const data = await fetchFiles(path)
      files.value = data
      currentPath.value = path
    } catch (e) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  function navigateTo(path) {
    loadDirectory(path)
  }

  function navigateUp() {
    const parts = currentPath.value.split('/').filter(Boolean)
    if (parts.length === 0) return
    parts.pop()
    const newPath = parts.length === 0 ? '/' : '/' + parts.join('/')
    loadDirectory(newPath)
  }

  function goHome() {
    loadDirectory('/')
  }

  function toggleSelect(name) {
    if (selectedIds.value.has(name)) {
      selectedIds.value.delete(name)
    } else {
      selectedIds.value.add(name)
    }
  }

  function selectAll() {
    files.value.forEach(f => selectedIds.value.add(f.name))
  }

  function deselectAll() {
    selectedIds.value.clear()
  }

  function invertSelection() {
    files.value.forEach(f => {
      if (selectedIds.value.has(f.name)) {
        selectedIds.value.delete(f.name)
      } else {
        selectedIds.value.add(f.name)
      }
    })
  }

  function setViewMode(mode) {
    viewMode.value = mode
  }

  function setSearchQuery(query) {
    searchQuery.value = query
  }

  async function openFile(file) {
    if (file.type === 'dir') {
      const newPath = currentPath.value === '/'
        ? `/${file.name}`
        : `${currentPath.value}/${file.name}`
      loadDirectory(newPath)
    } else {
      const filePath = currentPath.value === '/'
        ? `/${file.name}`
        : `${currentPath.value}/${file.name}`
      window.open(filePath, '_blank')
    }
  }

  async function previewTextFile(file) {
    const filePath = currentPath.value === '/'
      ? `/${file.name}`
      : `${currentPath.value}/${file.name}`

    try {
      const content = await fetchFileContent(filePath)
      previewFile.value = file
      previewContent.value = content
    } catch (e) {
      error.value = `预览失败: ${e.message}`
    }
  }

  function closePreview() {
    previewFile.value = null
    previewContent.value = null
  }

  function downloadFile(file) {
    const filePath = currentPath.value === '/'
      ? `/${file.name}`
      : `${currentPath.value}/${file.name}`
    const link = document.createElement('a')
    link.href = filePath
    link.download = file.name
    link.click()
  }

  async function downloadSelected() {
    const selected = files.value.filter(f => selectedIds.value.has(f.name))
    for (let i = 0; i < selected.length; i++) {
      downloadFile(selected[i])
      if (i < selected.length - 1) {
        await new Promise(r => setTimeout(r, 1000))
      }
    }
  }

  function isSelected(name) {
    return selectedIds.value.has(name)
  }

  function setKeyboardIndex(index) {
    keyboardIndex.value = index
  }

  function getFileByIndex(index) {
    return sortedFiles.value[index]
  }

  return {
    files,
    currentPath,
    loading,
    error,
    selectedIds,
    viewMode,
    searchQuery,
    previewContent,
    previewFile,
    keyboardIndex,
    filteredFiles,
    sortedFiles,
    selectedCount,
    totalCount,
    allSelected,
    someSelected,
    loadDirectory,
    navigateTo,
    navigateUp,
    goHome,
    toggleSelect,
    selectAll,
    deselectAll,
    invertSelection,
    setViewMode,
    setSearchQuery,
    openFile,
    previewTextFile,
    closePreview,
    downloadFile,
    downloadSelected,
    isSelected,
    setKeyboardIndex,
    getFileByIndex
  }
})
