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
