const API_BASE = ''

export async function fetchFiles(path) {
  const normalizedPath = path.startsWith('/') ? path : '/' + path
  const res = await fetch(normalizedPath, {
    headers: { 'Accept': 'application/json' }
  })

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`)
  }

  const contentType = res.headers.get('content-type') || ''
  if (!contentType.includes('application/json')) {
    throw new Error('Server did not return JSON response')
  }

  return res.json()
}

export async function fetchFileContent(path) {
  const res = await fetch(path)

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`)
  }

  const text = await res.text()
  if (text.length > 1024 * 1024) {
    throw new Error('File size exceeds 1MB preview limit')
  }

  return text
}

/**
 * 下载文件并支持进度跟踪
 * @param {string} path - 文件路径
 * @param {string} filename - 文件名
 * @param {Function} onProgress - 进度回调函数 (receivedBytes, totalBytes, percentage)
 */
export async function downloadFileWithProgress(path, filename, onProgress) {
  const res = await fetch(path)

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`)
  }

  const contentLength = res.headers.get('content-length')
  const total = parseInt(contentLength, 10)
  
  if (!res.body) {
    throw new Error('ReadableStream not supported')
  }

  const reader = res.body.getReader()
  const chunks = []
  let receivedLength = 0

  while (true) {
    const { done, value } = await reader.read()

    if (done) break

    chunks.push(value)
    receivedLength += value.length

    if (onProgress && total) {
      const percentage = (receivedLength / total) * 100
      onProgress(receivedLength, total, percentage)
    }
  }

  // 合并所有块
  const blob = new Blob(chunks)
  
  // 创建下载链接
  const url = window.URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  
  // 清理
  setTimeout(() => {
    document.body.removeChild(a)
    window.URL.revokeObjectURL(url)
  }, 100)
}

/**
 * 检查服务器是否支持 Range 请求
 * @param {string} path - 文件路径
 */
export async function checkRangeSupport(path) {
  const res = await fetch(path, { method: 'HEAD' })
  const acceptRanges = res.headers.get('accept-ranges')
  return acceptRanges === 'bytes'
}
