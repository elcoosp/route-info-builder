import { defineDocsConfig } from '@elcoosp/docs-toolkit/config';

export default await defineDocsConfig({
  llmsTxt: {},
  site: 'https://route-info-builder.vercel.app',
  domain: 'route-info-builder.vercel.app',
  title: "Route Info Builder",
  description: "Build-time route scanner for Axum-like frameworks: type-safe Rust link enums, TypeScript clients with TanStack Query hooks, OpenAPI 3.1 skeletons and JSON route maps.",
  sidebar: [],
  extraStarlight: {
                        title: 'Route Info Builder',
                        description:
                                'Build-time route scanner for Axum-like frameworks: type-safe Rust link enums, TypeScript clients with TanStack Query hooks, OpenAPI 3.1 skeletons and JSON route maps.',
                        favicon: '/favicon.png',
                        social: [
                                {
                                        icon: 'github',
                                        label: 'GitHub', translations: { fr: "GitHub" },
                                        href: 'https://github.com/elcoosp/route-info-builder'
                                },
                        ],
                        editLink: {
                                baseUrl: 'https://github.com/elcoosp/route-info-builder/edit/main/'
                        },

                                                // The 404 page is a content page (src/content/docs/404.md) rendered
                        // through the [...slug] route; Starlight's injected 404 route is
                        // disabled to avoid the two routes conflicting.
                        disable404Route: true,
                        sidebar: [
                                {
                                        label: 'Tutorials', translations: { fr: 'Tutoriels' },
                                        items: ['tutorials/getting-started']
                                },
                                {
                                        label: 'How-to guides', translations: { fr: 'Guides pratiques' },
                                        items: [
                                                'guides/generate-from-cli',
                                                'guides/configure-naming',
                                                'guides/wire-authentication',
                                                'guides/troubleshoot-scanning',
                                        ]
                                },
                                {
                                        label: 'Reference', translations: { fr: 'Référence' },
                                        items: [
                                                'reference/configuration',
                                                'reference/cli',
                                                'reference/api',
                                                'reference/route-info-json',
                                        ]
                                },
                                {
                                        label: 'Explanation', translations: { fr: 'Explication' },
                                        items: [
                                                'explanation/scanning-pipeline',
                                                'explanation/naming-and-collisions',
                                                'explanation/design-decisions',
                                        ]
                                },
                        ],
                        expressiveCode: {
                                themes: ['github-light', 'github-dark']
                        }
                },
});
