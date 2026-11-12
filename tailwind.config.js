/** @type {import('tailwindcss').Config} */
export default {
  content: ['./web/index.html', './web/src/**/*.{svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        ink: '#1a1a2e',
        card: '#16213e',
        accent: '#0f3460',
        signal: '#e94560',
        paper: '#eaeaea',
        mint: '#4ecca3'
      },
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', '-apple-system', 'Segoe UI', 'sans-serif'],
        mono: ['ui-monospace', 'SFMono-Regular', 'Cascadia Code', 'Consolas', 'monospace']
      },
      animation: {
        'wave-slow': 'wave 7s ease-in-out infinite',
        'wave-fast': 'wave 4s ease-in-out infinite',
        scan: 'scan 8s linear infinite',
        float: 'float 6s ease-in-out infinite'
      },
      keyframes: {
        wave: {
          '0%, 100%': { transform: 'scaleY(0.35)' },
          '50%': { transform: 'scaleY(1)' }
        },
        scan: {
          '0%': { transform: 'translateY(-20%)' },
          '100%': { transform: 'translateY(120%)' }
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-8px)' }
        }
      }
    }
  },
  plugins: []
};
