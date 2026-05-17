import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
	plugins: [react()],
	server: {
		host: "127.0.0.1",
		port: 4173,
		proxy: {
			"/api": {
				target: process.env.VITE_API_TARGET ?? "http://127.0.0.1:3000",
				changeOrigin: true,
				rewrite: (path) => path.replace(/^\/api/, ""),
			},
		},
	},
	test: {
		environment: "jsdom",
		globals: true,
		setupFiles: "./vitest.setup.ts",
		include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
		exclude: ["e2e/**"],
	},
});
