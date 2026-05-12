'use client'

export function MeshGradient() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <div
        className="absolute h-[60%] w-[50%] rounded-full opacity-[0.12] blur-[120px] dark:opacity-[0.08]"
        style={{
          background: 'var(--color-fd-primary)',
          top: '10%',
          left: '20%',
          animation: 'mesh-a 20s ease-in-out infinite',
        }}
      />
      <div
        className="absolute h-[45%] w-[40%] rounded-full opacity-[0.08] blur-[100px] dark:opacity-[0.06]"
        style={{
          background: 'var(--color-fd-primary)',
          top: '40%',
          right: '10%',
          animation: 'mesh-b 25s ease-in-out infinite',
        }}
      />
      <div
        className="absolute h-[35%] w-[35%] rounded-full opacity-[0.06] blur-[80px] dark:opacity-[0.04]"
        style={{
          background: 'var(--color-fd-foreground)',
          bottom: '10%',
          left: '40%',
          animation: 'mesh-c 18s ease-in-out infinite',
        }}
      />
    </div>
  )
}
