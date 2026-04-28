const KEY = 'cip.serverBaseUrl'
const DEFAULT = 'http://127.0.0.1:8788'

export function getServerBaseUrl(): string {
  try {
    const v = (window.localStorage.getItem(KEY) ?? '').trim()
    if (v) return v.replace(/\/+$/, '')
  } catch {
  }
  return DEFAULT
}

export function setServerBaseUrl(input: string) {
  const v = (input ?? '').trim().replace(/\/+$/, '')
  try {
    if (!v) window.localStorage.removeItem(KEY)
    else window.localStorage.setItem(KEY, v)
  } catch {
  }
}

export function getDefaultServerBaseUrl(): string {
  return DEFAULT
}
