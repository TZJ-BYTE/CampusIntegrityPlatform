import { getServerBaseUrl } from './serverBaseUrl'

export function resolveContentAssetUrl(input?: string | null): string | undefined {
  const base = getServerBaseUrl()
  const v = (input ?? '').trim()
  if (!v) return undefined
  if (/^https?:\/\//i.test(v)) return v
  if (v.startsWith('/')) return `${base}${v}`
  return `${base}/${v}`
}
