/**
 * Generates public/llms.txt and public/llms-full.txt so AI agents can
 * discover the documentation. Runs as the `prebuild` script, so both files
 * are copied into dist/ by every build.
 *
 * - llms.txt: site header plus one entry per page (title, URL, description).
 * - llms-full.txt: the same header followed by the full Markdown body of
 *   every page (frontmatter stripped, MDX import lines removed).
 */
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';

const DOCS = 'src/content/docs';
const SITE_TITLE = 'Route Info Builder';
const SITE_DESCRIPTION =
	'Build-time route scanner for Axum-like frameworks: generates type-safe Rust link enums, TypeScript clients with TanStack Query hooks, OpenAPI 3.1 skeletons and JSON route maps.';

/** Collects every .md/.mdx file under `dir`, sorted, as absolute paths. */
async function collect(dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	const files = [];
	for (const entry of entries) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) files.push(...(await collect(full)));
		else if (/\.(md|mdx)$/.test(entry.name)) files.push(full);
	}
	return files.sort();
}

/** Splits YAML-ish frontmatter from the body and extracts title/description. */
function parse(file, text) {
	const match = /^---\n([\s\S]*?)\n---\n?/.exec(text);
	const meta = {};
	if (match) {
		for (const line of match[1].split('\n')) {
			const field = /^(title|description):\s*(.+)$/.exec(line);
			if (field) meta[field[1]] = field[2].replace(/^["']|["']$/g, '');
		}
	}
	let body = match ? text.slice(match[0].length) : text;
	body = body
		.split('\n')
		.filter((line) => !/^import\s\s/.test(line) && !/^import\s.*\sfrom\s/.test(line))
		.join('\n')
		.trim();
	const rel = path.relative(DOCS, file).replace(/\\/g, '/');
	const url =
		rel === 'index.mdx' || rel === 'index.md'
			? '/'
			: `/${rel.replace(/\.mdx?$/, '')}/`;
	return { meta, body, url };
}

const files = await collect(DOCS);
const pages = [];
for (const file of files) {
	const rel = path.relative(DOCS, file).replace(/\\/g, '/');
	if (rel === '404.md' || rel === '404.mdx') continue; // utility page
	const text = await readFile(file, 'utf8');
	pages.push(parse(file, text));
}

const header = `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n`;

let llms = `${header}\n`;
let full = `${header}\n`;
for (const page of pages) {
	const title = page.meta.title ?? page.url;
	const description = page.meta.description ?? '';
	if (page.url === '/') {
		llms += `- [${title}](${page.url}): ${description}\n`;
	} else {
		llms += `- [${title}](${page.url}): ${description}\n`;
		full += `\n\n---\n\n# ${title}\n\nSource: ${page.url}\n\n${page.body}`;
	}
}

await mkdir('public', { recursive: true });
await writeFile('public/llms.txt', `${llms}\n`);
await writeFile('public/llms-full.txt', `${full}\n`);
console.log(`llms.txt + llms-full.txt generated from ${pages.length} pages.`);
