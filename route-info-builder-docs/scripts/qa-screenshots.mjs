/**
 * Visual QA gate: screenshots every built route in two viewports and two
 * themes and fails on console errors, page errors, HTTP >= 400, horizontal
 * overflow, missing/duplicate <h1>, blank <main>, theme-application failure,
 * broken images, a non-PNG logo, and a logo rendered larger than its native
 * pixels.
 *
 * Run against a production preview (never the dev server):
 *
 *   pnpm build
 *   pnpm preview --port 4321 &
 *   pnpm qa
 *
 * dist/404.html is screenshotted once but excluded from the content checks
 * (a 404 page is intentionally short).
 */
import { chromium } from 'playwright';
import { readdir, mkdir, writeFile, stat } from 'node:fs/promises';
import path from 'node:path';

const BASE = process.env.QA_BASE_URL ?? 'http://localhost:4321';
const OUT = '.qa/screenshots';

async function walk(dir) {
        const entries = await readdir(dir, { withFileTypes: true });
        const files = [];
        for (const e of entries) {
                const p = path.join(dir, e.name);
                if (e.isDirectory()) files.push(...(await walk(p)));
                else if (e.name === 'index.html') files.push(p);
        }
        return files;
}

async function routes() {
        const files = await walk('dist');
        const list = files.map((f) => {
                const rel = path.relative('dist', path.dirname(f));
                // The root index.html yields '' (not '.') from path.relative.
                return '/' + (rel === '.' || rel === '' ? '' : rel + '/');
        });
        // Screenshot the 404 page once; it is excluded from content checks below.
        try {
                await stat('dist/404.html');
                list.push('/404.html');
        } catch {
                // No custom 404 page built; the default Starlight 404 is rendered on demand.
        }
        // Deterministic order with the homepage first.
        return [...new Set(list)].sort((a, b) =>
                a === '/' ? -1 : b === '/' ? 1 : a.localeCompare(b),
        );
}

const VIEWPORTS = {
        desktop: { width: 1440, height: 900 },
        mobile: { width: 390, height: 844 },
};

const failures = [];
const fail = (route, viewport, theme, kind, detail) =>
        failures.push({ route, viewport, theme, kind, detail });

const browser = await chromium.launch();
const all = await routes();
await mkdir(OUT, { recursive: true });

for (const route of all) {
        const isNotFound = route === '/404.html';
        for (const [vpName, viewport] of Object.entries(VIEWPORTS)) {
                for (const theme of ['light', 'dark']) {
                        const ctx = await browser.newContext({
                                viewport,
                                reducedMotion: 'reduce',
                        });
                        const page = await ctx.newPage();

                        page.on('console', (m) => {
                                if (m.type() === 'error') fail(route, vpName, theme, 'console', m.text());
                        });
                        page.on('pageerror', (e) =>
                                fail(route, vpName, theme, 'pageerror', e.message),
                        );
                        page.on('response', (r) => {
                                if (r.status() >= 400)
                                        fail(route, vpName, theme, 'http', `${r.status()} ${r.url()}`);
                        });

                        await page.addInitScript((t) => {
                                try {
                                        localStorage.setItem('starlight-theme', t);
                                } catch {}
                        }, theme);

                        await page.goto(BASE + route, { waitUntil: 'networkidle' });
                        await page.evaluate(() => document.fonts.ready);

                        const applied = await page.evaluate(
                                () => document.documentElement.dataset.theme,
                        );
                        if (applied !== theme) {
                                const toggle = page
                                        .locator("starlight-theme-select button, button[data-theme-toggle]")
                                        .first();
                                if (await toggle.count()) {
                                        await toggle.click();
                                        await page.waitForTimeout(150);
                                }
                        }

                        const probe = await page.evaluate(() => {
                                const main = document.querySelector('main');
                                const imgs = [...document.querySelectorAll('img')];
                                const logo = document.querySelector('.site-title img, header img');
                                return {
                                        theme: document.documentElement.dataset.theme,
                                        h1: document.querySelectorAll('h1').length,
                                        textLen: (main?.innerText ?? '').trim().length,
                                        overflow:
                                                document.documentElement.scrollWidth > window.innerWidth + 1,
                                        brokenImgs: imgs
                                                .filter((i) => !i.complete || i.naturalWidth === 0)
                                                .map((i) => i.currentSrc || i.src),
                                        logo: logo
                                                ? {
                                                                src: logo.currentSrc || logo.src,
                                                                naturalW: logo.naturalWidth,
                                                                naturalH: logo.naturalHeight,
                                                                renderW: logo.clientWidth,
                                                                renderH: logo.clientHeight,
                                                        }
                                                : null,
                                };
                        });

                        if (probe.theme !== theme)
                                fail(route, vpName, theme, 'theme', `applied=${probe.theme}`);
                        // 404 pages are intentionally short; skip the h1/blank checks there.
                        if (!isNotFound) {
                                if (probe.h1 !== 1) fail(route, vpName, theme, 'h1', `count=${probe.h1}`);
                                if (probe.textLen < 200)
                                        fail(route, vpName, theme, 'empty', `text=${probe.textLen}`);
                        }
                        if (probe.overflow)
                                fail(route, vpName, theme, 'overflow', 'horizontal scrollbar');
                        for (const src of probe.brokenImgs)
                                fail(route, vpName, theme, 'image', src);

                        if (probe.logo) {
                                if (!/\.png(\?|$)/i.test(probe.logo.src))
                                        fail(route, vpName, theme, 'logo-format', probe.logo.src);
                                if (
                                        probe.logo.renderW > probe.logo.naturalW + 1 ||
                                        probe.logo.renderH > probe.logo.naturalH + 1
                                )
                                        fail(
                                                route,
                                                vpName,
                                                theme,
                                                'logo-upscale',
                                                `render ${probe.logo.renderW}x${probe.logo.renderH} > natural ${probe.logo.naturalW}x${probe.logo.naturalH}`,
                                        );
                        }

                        const slug =
                                route === '/'
                                        ? 'index'
                                        : route.replace(/^\/|\/$/g, '').replace(/\//g, '_');
                        await page.screenshot({
                                path: path.join(OUT, `${slug}--${vpName}--${theme}.png`),
                                fullPage: true,
                        });
                        await ctx.close();
                }
        }
}

await browser.close();
await writeFile('.qa/failures.json', JSON.stringify(failures, null, 2));

if (failures.length) {
        console.error(`QA FAILED — ${failures.length} issue(s):`);
        for (const f of failures)
                console.error(
                        `  [${f.kind}] ${f.route} (${f.viewport}/${f.theme}): ${f.detail}`,
                );
        process.exit(1);
}
console.log(`QA PASSED — ${all.length} routes × 2 viewports × 2 themes`);
