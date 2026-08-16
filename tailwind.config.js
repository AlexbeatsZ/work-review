/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}', './index.html'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eef4ff',
          100: '#dbe7ff',
          200: '#bfd3ff',
          300: '#9bb9ff',
          400: '#7aa2f7',
          500: '#648cf0',
          600: '#5274dc',
          700: '#435db8',
          800: '#394e93',
          900: '#324276',
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
