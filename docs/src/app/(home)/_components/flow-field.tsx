'use client'

import { useEffect, useRef } from 'react'

function simplex2d(x: number, y: number): number {
  const F2 = 0.5 * (Math.sqrt(3) - 1)
  const G2 = (3 - Math.sqrt(3)) / 6
  const s = (x + y) * F2
  const i = Math.floor(x + s)
  const j = Math.floor(y + s)
  const t = (i + j) * G2
  const x0 = x - (i - t)
  const y0 = y - (j - t)
  const i1 = x0 > y0 ? 1 : 0
  const j1 = x0 > y0 ? 0 : 1
  const x1 = x0 - i1 + G2
  const y1 = y0 - j1 + G2
  const x2 = x0 - 1 + 2 * G2
  const y2 = y0 - 1 + 2 * G2
  const hash = (n: number) => { const h = Math.sin(n) * 43758.5453; return h - Math.floor(h) }
  const grad = (h: number, gx: number, gy: number) => { const a = h * Math.PI * 2; return Math.cos(a) * gx + Math.sin(a) * gy }
  let n0 = 0, n1 = 0, n2 = 0
  let t0 = 0.5 - x0 * x0 - y0 * y0
  if (t0 >= 0) { t0 *= t0; n0 = t0 * t0 * grad(hash(i * 127.1 + j * 311.7), x0, y0) }
  let t1 = 0.5 - x1 * x1 - y1 * y1
  if (t1 >= 0) { t1 *= t1; n1 = t1 * t1 * grad(hash((i + i1) * 127.1 + (j + j1) * 311.7), x1, y1) }
  let t2 = 0.5 - x2 * x2 - y2 * y2
  if (t2 >= 0) { t2 *= t2; n2 = t2 * t2 * grad(hash((i + 1) * 127.1 + (j + 1) * 311.7), x2, y2) }
  return 70 * (n0 + n1 + n2)
}

const VERT = /* glsl */ `
  attribute vec2 aPos;
  attribute float aAlpha;
  attribute float aHue;
  attribute float aSize;
  uniform vec2 uRes;
  varying float vAlpha;
  varying float vHue;
  void main() {
    vec2 ndc = (aPos / uRes) * 2.0 - 1.0;
    ndc.y = -ndc.y;
    gl_Position = vec4(ndc, 0.0, 1.0);
    gl_PointSize = aSize;
    vAlpha = aAlpha;
    vHue = aHue;
  }
`

const FRAG = /* glsl */ `
  precision highp float;
  varying float vAlpha;
  varying float vHue;
  uniform float uDark;
  void main() {
    float d = length(gl_PointCoord - 0.5) * 2.0;
    if (d > 1.0) discard;
    float soft = 1.0 - d * d;
    vec3 warmD = vec3(0.55, 0.45, 0.27);
    vec3 goldD = vec3(0.83, 0.66, 0.27);
    vec3 coolL = vec3(0.23, 0.31, 0.47);
    vec3 blueL = vec3(0.33, 0.42, 0.56);
    vec3 color = mix(
      mix(coolL, blueL, vHue),
      mix(warmD, goldD, vHue),
      uDark
    );
    float a = vAlpha * soft;
    gl_FragColor = vec4(color * a, a);
  }
`

const TRAIL = 12
const COUNT = 1200
const MAX_VERTS = COUNT * TRAIL
const FLOATS_PER = 5

interface Particle {
  trail: Float64Array
  head: number
  vx: number
  vy: number
  life: number
  maxLife: number
  hue: number
}

export function FlowField() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rafRef = useRef(0)
  const mouseRef = useRef({ x: -1, y: -1 })

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const gl = canvas.getContext('webgl', { alpha: true, antialias: true })
    if (!gl) return

    const compile = (type: number, src: string) => {
      const s = gl.createShader(type)!
      gl.shaderSource(s, src)
      gl.compileShader(s)
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
        console.error('shader compile:', gl.getShaderInfoLog(s))
      }
      return s
    }
    const prog = gl.createProgram()!
    gl.attachShader(prog, compile(gl.VERTEX_SHADER, VERT))
    gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, FRAG))
    gl.linkProgram(prog)
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.error('program link:', gl.getProgramInfoLog(prog))
    }
    gl.useProgram(prog)

    const aPos = gl.getAttribLocation(prog, 'aPos')
    const aAlpha = gl.getAttribLocation(prog, 'aAlpha')
    const aHue = gl.getAttribLocation(prog, 'aHue')
    const aSize = gl.getAttribLocation(prog, 'aSize')
    const uRes = gl.getUniformLocation(prog, 'uRes')
    const uDark = gl.getUniformLocation(prog, 'uDark')

    const buf = gl.createBuffer()!
    gl.bindBuffer(gl.ARRAY_BUFFER, buf)
    gl.bufferData(gl.ARRAY_BUFFER, MAX_VERTS * FLOATS_PER * 4, gl.DYNAMIC_DRAW)

    const stride = FLOATS_PER * 4
    gl.enableVertexAttribArray(aPos)
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, stride, 0)
    gl.enableVertexAttribArray(aAlpha)
    gl.vertexAttribPointer(aAlpha, 1, gl.FLOAT, false, stride, 8)
    gl.enableVertexAttribArray(aHue)
    gl.vertexAttribPointer(aHue, 1, gl.FLOAT, false, stride, 12)
    gl.enableVertexAttribArray(aSize)
    gl.vertexAttribPointer(aSize, 1, gl.FLOAT, false, stride, 16)

    gl.enable(gl.BLEND)
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA)

    const vertexData = new Float32Array(MAX_VERTS * FLOATS_PER)

    const dpr = devicePixelRatio
    let w = 0, h = 0

    const resize = () => {
      w = canvas.clientWidth
      h = canvas.clientHeight
      canvas.width = w * dpr
      canvas.height = h * dpr
      gl.viewport(0, 0, canvas.width, canvas.height)
      gl.uniform2f(uRes, w, h)
    }
    resize()
    addEventListener('resize', resize)

    const onMouse = (e: MouseEvent) => {
      const r = canvas.getBoundingClientRect()
      mouseRef.current = { x: e.clientX - r.left, y: e.clientY - r.top }
    }
    const onLeave = () => { mouseRef.current = { x: -1, y: -1 } }
    canvas.addEventListener('mousemove', onMouse)
    canvas.addEventListener('mouseleave', onLeave)

    const spawn = (): Particle => {
      const x = Math.random() * w
      const y = Math.random() * h
      const trail = new Float64Array(TRAIL * 2)
      for (let k = 0; k < TRAIL; k++) { trail[k * 2] = x; trail[k * 2 + 1] = y }
      return { trail, head: 0, vx: 0, vy: 0, life: 0, maxLife: 250 + Math.random() * 350, hue: Math.random() }
    }

    const particles: Particle[] = []
    for (let i = 0; i < COUNT; i++) {
      const p = spawn()
      p.life = Math.random() * p.maxLife
      particles.push(p)
    }

    let time = Math.random() * 1000
    let paused = false
    let last = performance.now()
    let acc = 0
    const dt = 1 / 60

    const onVis = () => {
      paused = document.hidden
      if (!paused) { last = performance.now(); rafRef.current = requestAnimationFrame(loop) }
    }
    document.addEventListener('visibilitychange', onVis)

    let darkVal = document.documentElement.classList.contains('dark') ? 1.0 : 0.0
    let darkTarget = darkVal
    const obs = new MutationObserver(() => {
      darkTarget = document.documentElement.classList.contains('dark') ? 1.0 : 0.0
    })
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })

    const tick = () => {
      time += 0.002
      const mx = mouseRef.current.x
      const my = mouseRef.current.y
      const hasM = mx >= 0 && my >= 0

      for (const p of particles) {
        const tx = p.trail[p.head * 2]
        const ty = p.trail[p.head * 2 + 1]
        const angle = simplex2d(tx * 0.0015 + time, ty * 0.0015 + time * 0.5) * Math.PI * 1.8
        let fx = Math.cos(angle) * 0.6
        let fy = Math.sin(angle) * 0.6

        if (hasM) {
          const dx = mx - tx, dy = my - ty
          const d = Math.sqrt(dx * dx + dy * dy)
          if (d < 200 && d > 1) { const f = (1 - d / 200) * 1.5; fx += (dx / d) * f; fy += (dy / d) * f }
        }

        p.vx = p.vx * 0.95 + fx * 0.05
        p.vy = p.vy * 0.95 + fy * 0.05
        p.head = (p.head + 1) % TRAIL
        p.trail[p.head * 2] = tx + p.vx
        p.trail[p.head * 2 + 1] = ty + p.vy
        p.life++

        const nx = p.trail[p.head * 2], ny = p.trail[p.head * 2 + 1]
        if (p.life >= p.maxLife || nx < -20 || nx > w + 20 || ny < -20 || ny > h + 20) {
          const s = spawn(); p.trail.set(s.trail); p.head = 0; p.vx = 0; p.vy = 0
          p.life = 0; p.maxLife = s.maxLife; p.hue = s.hue
        }
      }
    }

    const draw = () => {
      darkVal += (darkTarget - darkVal) * 0.05

      gl.clearColor(0, 0, 0, 0)
      gl.clear(gl.COLOR_BUFFER_BIT)
      gl.uniform1f(uDark, darkVal)

      let vi = 0
      for (const p of particles) {
        const lr = p.life / p.maxLife
        const fi = Math.min(lr * 4, 1)
        const fo = lr > 0.75 ? (1 - lr) / 0.25 : 1
        const base = fi * fo
        if (base <= 0.01) continue

        for (let i = 0; i < TRAIL; i++) {
          const idx = (p.head - i + TRAIL) % TRAIL
          const tf = 1 - i / TRAIL
          const a = base * tf * tf * (darkVal > 0.5 ? 0.5 : 0.35)
          if (a < 0.01) continue

          const off = vi * FLOATS_PER
          vertexData[off] = p.trail[idx * 2]
          vertexData[off + 1] = p.trail[idx * 2 + 1]
          vertexData[off + 2] = a
          vertexData[off + 3] = p.hue
          vertexData[off + 4] = (1 + tf * 2) * dpr
          vi++
        }
      }

      if (vi > 0) {
        gl.bufferSubData(gl.ARRAY_BUFFER, 0, vertexData.subarray(0, vi * FLOATS_PER))
        gl.drawArrays(gl.POINTS, 0, vi)
      }
    }

    const loop = () => {
      if (paused) return
      const now = performance.now()
      acc += Math.min((now - last) / 1000, 0.05)
      last = now

      let n = 0
      while (acc >= dt && n < 3) { tick(); acc -= dt; n++ }
      acc = Math.min(acc, dt)

      draw()
      rafRef.current = requestAnimationFrame(loop)
    }

    for (let i = 0; i < 60; i++) tick()
    draw()
    rafRef.current = requestAnimationFrame(loop)

    return () => {
      cancelAnimationFrame(rafRef.current)
      removeEventListener('resize', resize)
      canvas.removeEventListener('mousemove', onMouse)
      canvas.removeEventListener('mouseleave', onLeave)
      document.removeEventListener('visibilitychange', onVis)
      obs.disconnect()
    }
  }, [])

  return <canvas ref={canvasRef} className="absolute inset-0 size-full" />
}
