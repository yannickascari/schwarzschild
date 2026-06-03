import { defineConfig } from 'vite';

export default defineConfig({
    base: '/schwarzschild/',
    server: {
        fs: {
            allow: ['..']
        }
    },
    publicDir: 'public',
});