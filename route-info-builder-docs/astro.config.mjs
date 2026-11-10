// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import astroTakumi from 'astro-takumi';
import fs from 'node:fs';
import { makeRender } from './src/og-render.mjs';
import linksValidator from 'starlight-links-validator';

const __logoB64 = fs.readFileSync("./src/assets/logo.png").toString("base64");
const __render = makeRender({
  logoDataUrl: "data:image/png;base64," + __logoB64,
  accentHex: "#743e3e",
  accentHexA12: "#743e3e1a",
  domain: "route-info-builder.vercel.app",
  projectName: "Route Info Builder",
});

export default defineConfig({
	vite: {
		ssr: { external: ["jsdom", "astro-takumi"] },
	},
  site: 'https://route-info-builder.vercel.app',
        integrations: [
                starlight({
			routeMiddleware: "./src/routeData.ts",
                        title: 'Route Info Builder',
                        description:
                                'Build-time route scanner for Axum-like frameworks: type-safe Rust link enums, TypeScript clients with TanStack Query hooks, OpenAPI 3.1 skeletons and JSON route maps.',
                        favicon: '/favicon.png',
                        customCss: ['./src/styles/custom.css'],

                        logo: { src: './src/assets/logo.png' },
                        social: [
                                {
                                        icon: 'github',
                                        label: 'GitHub',
                                        href: 'https://github.com/elcoosp/route-info-builder',
                                },
                        ],
                        editLink: {
                                baseUrl: 'https://github.com/elcoosp/route-info-builder/edit/main/',
                        },
                        defaultLocale: 'root',
                        locales: {
                                root: { label: 'English', lang: 'en' },
                        },
                        // The 404 page is a content page (src/content/docs/404.md) rendered
                        // through the [...slug] route; Starlight's injected 404 route is
                        // disabled to avoid the two routes conflicting.
                        disable404Route: true,
                        sidebar: [
                                {
                                        label: 'Tutorials',
                                        items: ['tutorials/getting-started'],
                                },
                                {
                                        label: 'How-to guides',
                                        items: [
                                                'guides/generate-from-cli',
                                                'guides/configure-naming',
                                                'guides/wire-authentication',
                                                'guides/troubleshoot-scanning',
                                        ],
                                },
                                {
                                        label: 'Reference',
                                        items: [
                                                'reference/configuration',
                                                'reference/cli',
                                                'reference/api',
                                                'reference/route-info-json',
                                        ],
                                },
                                {
                                        label: 'Explanation',
                                        items: [
                                                'explanation/scanning-pipeline',
                                                'explanation/naming-and-collisions',
                                                'explanation/design-decisions',
                                        ],
                                },
                        ],
                        expressiveCode: {
                                themes: ['github-light', 'github-dark'],
                        },
                }),
                linksValidator(),
        
		astroTakumi({
			options: {
				domBackend: "linkedom",
				fonts: [fs.readFileSync("./node_modules/@fontsource/roboto/files/roboto-latin-400-normal.woff2")],
				width: 1200,
				height: 630,
				format: "png",
			},
			render: __render,
		}),
	],
});
