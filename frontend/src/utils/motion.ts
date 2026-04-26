import { gsap } from 'gsap'

export function prefersReducedMotion() {
  try {
    return window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches ?? false
  } catch {
    return false
  }
}

export function animateIn(el: Element, opts?: { from?: gsap.TweenVars; to?: gsap.TweenVars }) {
  if (prefersReducedMotion()) return
  gsap.fromTo(
    el,
    { opacity: 0, y: 8, ...(opts?.from ?? {}) } as gsap.TweenVars,
    { opacity: 1, y: 0, duration: 0.26, ease: 'power3.out', ...(opts?.to ?? {}) } as gsap.TweenVars,
  )
}

export function animatePop(el: Element) {
  if (prefersReducedMotion()) return
  gsap.fromTo(el, { scale: 0.98 }, { scale: 1, duration: 0.22, ease: 'power3.out' })
}

export function animateOut(el: Element, to?: gsap.TweenVars) {
  if (prefersReducedMotion()) return Promise.resolve()
  return new Promise<void>((resolve) => {
    gsap.to(el, { opacity: 0, y: 6, duration: 0.2, ease: 'power3.in', onComplete: resolve, ...(to ?? {}) })
  })
}

export function animateStaggerIn(
  root: Element,
  selector: string,
  opts?: { from?: gsap.TweenVars; to?: gsap.TweenVars; stagger?: number },
) {
  if (prefersReducedMotion()) return
  const targets = Array.from(root.querySelectorAll(selector))
  if (targets.length === 0) return
  gsap.fromTo(
    targets,
    { opacity: 0, y: 8, ...(opts?.from ?? {}) } as gsap.TweenVars,
    { opacity: 1, y: 0, duration: 0.32, ease: 'power3.out', stagger: opts?.stagger ?? 0.04, ...(opts?.to ?? {}) } as gsap.TweenVars,
  )
}
