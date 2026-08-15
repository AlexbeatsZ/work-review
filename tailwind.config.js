/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}', './index.html'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#faf1eb',
          100: '#f4e2d7',
          200: '#e8c4b0',
          300: '#d99d7d',
          400: '#ce7952',
          500: '#c5663d',
          600: '#aa5330',
          700: '#884129',
          800: '#703825',
          900: '#5d3022',
        },
      },
      fontFamily: {
        sans: ['Segoe UI Variable Text', 'PingFang SC', 'Noto Sans CJK SC', 'Microsoft YaHei', 'system-ui', 'sans-serif'],
        mono: ['Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
};
