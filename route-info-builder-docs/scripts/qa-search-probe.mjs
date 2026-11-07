/**
 * Probes the Pagefind search modal: opens it via the header search button,
 * types a domain term, and reports the result titles.
 */
import { chromium } from 'playwright';

const BASE = process.env.QA_BASE_URL ?? 'http://localhost:4321';

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto(BASE + '/', { waitUntil: 'networkidle' });

// Starlight's header search button carries aria-keyshortcuts="Control K".
const searchButton = page.locator('button[aria-keyshortcuts], [aria-label*="earch" i]').first();
await searchButton.click();

const panel = page.locator('dialog[open], [role="dialog"], starlight-search-modal');
await panel.waitFor({ state: 'visible', timeout: 10_000 });

const input = page.locator('dialog input, starlight-search-modal input, [role="dialog"] input').first();
await input.fill('collision');
await page.waitForTimeout(1500);
const results = await page.locator('mark').allInnerTexts();
const titles = await page.locator('.pagefind-ui__result-title a, dialog a').allInnerTexts();

console.log(
	JSON.stringify({ modalOpened: true, marks: results.slice(0, 6), linkTexts: titles.slice(0, 6) }, null, 2),
);
await browser.close();
