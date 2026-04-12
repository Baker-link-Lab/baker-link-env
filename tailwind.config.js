/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        bkl: {
          base:            '#111014',
          sidebar:         '#18161c',
          surface:         '#1e1c24',
          'surface-hover': '#26232e',
          'surface-inset': '#0d0b10',
          border:          '#2a2733',
          'border-accent': 'rgba(224, 122, 31, 0.25)',
          text:            '#e8e4ef',
          'text-muted':    '#8a8494',
          'text-faint':    '#5c576a',
          orange:          '#e07a1f',
          'orange-light':  '#f0963f',
          'orange-dark':   '#b85d16',
          'orange-glow':   'rgba(224, 122, 31, 0.15)',
          green:           '#3dd68c',
          red:             '#ef4444',
          gray:            '#6b7280',
        },
      },
      fontFamily: {
        sans: ['"Noto Sans JP"', '"Yu Gothic UI"', '"Segoe UI"', 'sans-serif'],
        mono: ['Consolas', '"SF Mono"', '"Fira Code"', 'monospace'],
      },
      keyframes: {
        'pulse-green': {
          '0%, 100%': { boxShadow: '0 0 4px rgba(61, 214, 140, 0.3)' },
          '50%':      { boxShadow: '0 0 12px rgba(61, 214, 140, 0.6)' },
        },
        'slide-in': {
          from: { opacity: '0', transform: 'translateY(-10px)' },
          to:   { opacity: '1', transform: 'translateY(0)' },
        },
        'fade-in': {
          from: { opacity: '0' },
          to:   { opacity: '1' },
        },
        'scale-in': {
          from: { opacity: '0', transform: 'scale(0.95)' },
          to:   { opacity: '1', transform: 'scale(1)' },
        },
        'splash-orb-1': {
          '0%':   { transform: 'translate(-80px, -40px) scale(1)' },
          '100%': { transform: 'translate(60px, 30px) scale(1.15)' },
        },
        'splash-orb-2': {
          '0%':   { transform: 'translate(100px, 50px) scale(1.1)' },
          '100%': { transform: 'translate(-60px, -20px) scale(0.9)' },
        },
        'splash-shimmer': {
          '0%':   { transform: 'translateX(-200%)' },
          '100%': { transform: 'translateX(200%)' },
        },
        'splash-logo-enter': {
          '0%':   { opacity: '0', transform: 'scale(0.7) translateY(10px)', filter: 'blur(8px)' },
          '100%': { opacity: '1', transform: 'scale(1) translateY(0)', filter: 'blur(0)' },
        },
        'splash-text-enter': {
          '0%':   { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        'splash-fade-in': {
          '0%':   { opacity: '0' },
          '100%': { opacity: '1' },
        },
      },
      animation: {
        'pulse-green':      'pulse-green 2s ease-in-out infinite',
        'slide-in':         'slide-in 300ms ease-out',
        'fade-in':          'fade-in 200ms ease-out',
        'scale-in':         'scale-in 200ms ease-out',
        'splash-orb-1':     'splash-orb-1 3s ease-in-out infinite alternate',
        'splash-orb-2':     'splash-orb-2 3s ease-in-out infinite alternate',
        'splash-shimmer':   'splash-shimmer 2s ease-in-out infinite',
        'splash-logo':      'splash-logo-enter 800ms cubic-bezier(0.16, 1, 0.3, 1) both 200ms',
        'splash-text-1':    'splash-text-enter 600ms ease-out both 600ms',
        'splash-text-2':    'splash-text-enter 600ms ease-out both 800ms',
        'splash-fade-in':   'splash-fade-in 600ms ease-out',
      },
    },
  },
  plugins: [],
}
