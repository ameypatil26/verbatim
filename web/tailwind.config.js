import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));

/** @type {import('tailwindcss').Config} */
export default {
  content: [`${root}index.html`, `${root}src/**/*.{svelte,ts}`],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        ink: '#1a1a2e',
        'ink-deep': '#0e1426',
        card: '#16213e',
        accent: '#0f3460',
        signal: '#e94560',
        paper: '#eaeaea',
        muted: '#8c98ab',
        mint: '#4ecca3',
        sand: '#f3d9a4'
      },
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', '-apple-system', 'Segoe UI', 'sans-serif'],
        mono: ['Cascadia Code', 'SFMono-Regular', 'JetBrains Mono', 'Consolas', 'ui-monospace', 'monospace']
      },
      boxShadow: {
        glow: '0 0 60px rgba(78, 204, 163, 0.12)',
        signal: '0 0 45px rgba(233, 69, 96, 0.16)',
        card: '0 24px 70px rgba(0, 0, 0, 0.28)'
      },
      animation: {
        'wave-slow': 'wave 7s ease-in-out infinite',
        'wave-fast': 'wave 4s ease-in-out infinite',
        scan: 'scan 7s linear infinite',
        float: 'float 6s ease-in-out infinite',
        pulseSoft: 'pulseSoft 3s ease-in-out infinite'
      },
      keyframes: {
        wave: {
          '0%, 100%': { transform: 'scaleY(0.35)' },
          '50%': { transform: 'scaleY(1)' }
        },
        scan: {
          '0%': { transform: 'translateY(-10%)' },
          '100%': { transform: 'translateY(120%)' }
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-8px)' }
        },
        pulseSoft: {
          '0%, 100%': { opacity: '0.35' },
          '50%': { opacity: '0.8' }
        }
      }
    }
  },
  plugins: []
};
