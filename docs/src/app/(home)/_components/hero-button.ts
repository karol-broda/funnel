import { tv } from 'tailwind-variants'

export const heroButton = tv({
  base: 'flex items-center justify-center gap-2 rounded-full px-6 py-2.5 text-sm font-medium ring-1 ring-inset backdrop-blur transition-all',
  variants: {
    variant: {
      primary:
        'bg-fd-primary/90 text-fd-primary-foreground ring-fd-primary/20 hover:bg-fd-primary hover:ring-fd-primary/40',
      secondary:
        'bg-fd-foreground/[0.06] text-fd-foreground/70 ring-fd-foreground/[0.08] hover:bg-fd-foreground/[0.12] hover:text-fd-foreground/90',
    },
  },
})
