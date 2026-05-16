import { createCanvas, GlobalFonts } from "@napi-rs/canvas";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const width = 1600;
const height = 620;
const trail = 24;
const count = 1200;
const frames = 150;
const output = join(process.cwd(), "public/readme-hero.png");

type Particle = {
  trail: Float64Array;
  head: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  hue: number;
};

function mulberry32(seed: number) {
  return () => {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function simplex2d(x: number, y: number): number {
  const F2 = 0.5 * (Math.sqrt(3) - 1);
  const G2 = (3 - Math.sqrt(3)) / 6;
  const s = (x + y) * F2;
  const i = Math.floor(x + s);
  const j = Math.floor(y + s);
  const t = (i + j) * G2;
  const x0 = x - (i - t);
  const y0 = y - (j - t);
  const i1 = x0 > y0 ? 1 : 0;
  const j1 = x0 > y0 ? 0 : 1;
  const x1 = x0 - i1 + G2;
  const y1 = y0 - j1 + G2;
  const x2 = x0 - 1 + 2 * G2;
  const y2 = y0 - 1 + 2 * G2;
  const hash = (n: number) => {
    const h = Math.sin(n) * 43758.5453;
    return h - Math.floor(h);
  };
  const grad = (h: number, gx: number, gy: number) => {
    const a = h * Math.PI * 2;
    return Math.cos(a) * gx + Math.sin(a) * gy;
  };

  let n0 = 0;
  let n1 = 0;
  let n2 = 0;

  let t0 = 0.5 - x0 * x0 - y0 * y0;
  if (t0 >= 0) {
    t0 *= t0;
    n0 = t0 * t0 * grad(hash(i * 127.1 + j * 311.7), x0, y0);
  }

  let t1 = 0.5 - x1 * x1 - y1 * y1;
  if (t1 >= 0) {
    t1 *= t1;
    n1 = t1 * t1 * grad(hash((i + i1) * 127.1 + (j + j1) * 311.7), x1, y1);
  }

  let t2 = 0.5 - x2 * x2 - y2 * y2;
  if (t2 >= 0) {
    t2 *= t2;
    n2 = t2 * t2 * grad(hash((i + 1) * 127.1 + (j + 1) * 311.7), x2, y2);
  }

  return 70 * (n0 + n1 + n2);
}

function mix(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

function particleColor(hue: number, alpha: number) {
  const r = Math.round(mix(0.55, 0.83, hue) * 255);
  const g = Math.round(mix(0.45, 0.66, hue) * 255);
  const b = Math.round(mix(0.27, 0.27, hue) * 255);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

const rand = mulberry32(0xf00d2026);

function spawn(): Particle {
  const x = rand() * width;
  const y = rand() * height;
  const points = new Float64Array(trail * 2);

  for (let i = 0; i < trail; i++) {
    points[i * 2] = x;
    points[i * 2 + 1] = y;
  }

  return {
    trail: points,
    head: 0,
    vx: 0,
    vy: 0,
    life: rand() * 400,
    maxLife: 250 + rand() * 350,
    hue: rand(),
  };
}

const particles = Array.from({ length: count }, spawn);
let time = 112.35;

function tick() {
  time += 0.002;

  for (const p of particles) {
    const tx = p.trail[p.head * 2];
    const ty = p.trail[p.head * 2 + 1];
    const angle =
      simplex2d(tx * 0.0015 + time, ty * 0.0015 + time * 0.5) *
      Math.PI *
      1.8;
    const fx = Math.cos(angle) * 0.6;
    const fy = Math.sin(angle) * 0.6;

    p.vx = p.vx * 0.95 + fx * 0.05;
    p.vy = p.vy * 0.95 + fy * 0.05;
    p.head = (p.head + 1) % trail;
    p.trail[p.head * 2] = tx + p.vx;
    p.trail[p.head * 2 + 1] = ty + p.vy;
    p.life++;

    const nx = p.trail[p.head * 2];
    const ny = p.trail[p.head * 2 + 1];
    if (
      p.life >= p.maxLife ||
      nx < -50 ||
      nx > width + 50 ||
      ny < -50 ||
      ny > height + 50
    ) {
      const next = spawn();
      p.trail.set(next.trail);
      p.head = next.head;
      p.vx = next.vx;
      p.vy = next.vy;
      p.life = 0;
      p.maxLife = next.maxLife;
      p.hue = next.hue;
    }
  }
}

for (let i = 0; i < frames; i++) {
  tick();
}

GlobalFonts.registerFromPath(
  join(process.cwd(), "public/dm-serif-display-latin-400-normal.woff2"),
  "DM Serif Display",
);

const canvas = createCanvas(width, height);
const ctx = canvas.getContext("2d");

ctx.fillStyle = "#182238";
ctx.fillRect(0, 0, width, height);

ctx.globalCompositeOperation = "source-over";
for (const p of particles) {
  const lr = p.life / p.maxLife;
  const fadeIn = Math.min(lr * 4, 1);
  const fadeOut = lr > 0.75 ? (1 - lr) / 0.25 : 1;
  const base = Math.max(0, fadeIn * fadeOut);
  if (base <= 0.01) continue;

  ctx.strokeStyle = particleColor(p.hue, base * 0.1);
  ctx.lineWidth = 1;
  ctx.lineCap = "round";
  ctx.beginPath();
  for (let i = trail - 1; i >= 0; i--) {
    const idx = (p.head - i + trail) % trail;
    const x = p.trail[idx * 2];
    const y = p.trail[idx * 2 + 1];
    if (i === trail - 1) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();

  for (let i = trail - 1; i >= 0; i--) {
    const idx = (p.head - i + trail) % trail;
    const tf = 1 - i / trail;
    const alpha = base * tf * tf * 0.46;
    if (alpha < 0.01) continue;

    const x = p.trail[idx * 2];
    const y = p.trail[idx * 2 + 1];
    const radius = 0.55 + tf * 1.35;
    ctx.fillStyle = particleColor(p.hue, alpha);
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fill();
  }
}

const shade = ctx.createRadialGradient(
  width / 2,
  height / 2,
  width * 0.05,
  width / 2,
  height / 2,
  width * 0.26,
);
shade.addColorStop(0, "rgba(24, 34, 56, 0.82)");
shade.addColorStop(0.65, "rgba(24, 34, 56, 0.22)");
shade.addColorStop(1, "rgba(24, 34, 56, 0)");
ctx.fillStyle = shade;
ctx.fillRect(0, 0, width, height);

ctx.textAlign = "center";
ctx.textBaseline = "middle";
ctx.fillStyle = "rgba(240, 235, 227, 0.96)";
ctx.font = "174px 'DM Serif Display'";
ctx.fillText("funnel", width / 2, height / 2 - 18);

ctx.font = "24px sans-serif";
ctx.fillStyle = "rgba(240, 235, 227, 0.62)";
ctx.fillText("SELF-HOSTED TUNNELS OVER QUIC", width / 2, height / 2 + 112);

mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, await canvas.encode("png"));
console.log(`wrote ${output}`);
