// Generate src/data/elements.json, the element table of the Elements
// reference, from the Rust source, so it lists exactly what main supports.
//
//   node scripts/gen-reference.mjs           # write src/data/elements.json
//   node scripts/gen-reference.mjs --check   # fail if it is out of date
//
// It reads, from the repository:
//   bindings/rust/openui/src/element.rs               tag_definition: the tags
//   bindings/rust/openui-engine/src/lib.rs            create_native_element: default display
//   bindings/rust/openui-engine/src/accessibility.rs  inferred_role: default role
//   bindings/rust/openui-engine/src/interaction.rs    control and input types
//
// The style properties table (src/data/properties.json) needs Rust: it is
// written by site/tools/style-table (see site/README.md).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = join(dirname(fileURLToPath(import.meta.url)), '..');
const repo = join(site, '..');
const read = (path) => readFileSync(join(repo, path), 'utf8').replace(/\r\n/g, '\n');

/** The body of `fn name(` up to the matching closing brace. */
function functionBody(source, signature) {
  const start = source.indexOf(signature);
  if (start < 0) throw new Error(`cannot find ${signature}`);
  let depth = 0;
  for (let i = source.indexOf('{', start); i < source.length; i++) {
    if (source[i] === '{') depth++;
    else if (source[i] === '}' && --depth === 0) return source.slice(start, i + 1);
  }
  throw new Error(`unterminated ${signature}`);
}

function elements() {
  const tags = functionBody(read('bindings/rust/openui/src/element.rs'), 'fn tag_definition(');
  const rows = [];
  // Arms look like `"a" | "b" => (T::Kind, None | Some(Display::X)),`.
  const armPattern = /((?:"[a-z0-9]+"\s*\|?\s*)+)=>\s*\(T::(\w+),\s*(None|Some\(Display::(\w+)\))\)/g;
  for (const m of tags.matchAll(armPattern)) {
    const names = [...m[1].matchAll(/"([a-z0-9]+)"/g)].map((x) => x[1]);
    for (const tag of names) rows.push({ tag, kind: m[2], displayOverride: m[4] ?? null });
  }

  const engine = read('bindings/rust/openui-engine/src/lib.rs');
  const ctor = functionBody(engine, 'pub fn create_native_element(');
  const display = {};
  for (const m of ctor.matchAll(/((?:\s*\|?\s*ElementTag::\w+)+)\s*=>\s*Display::(\w+)/g)) {
    for (const k of m[1].matchAll(/ElementTag::(\w+)/g)) display[k[1]] = m[2];
  }

  const a11y = read('bindings/rust/openui-engine/src/accessibility.rs');
  const roleFn = functionBody(a11y, 'fn inferred_role(');
  const tagMatch = roleFn.slice(roleFn.indexOf('match tag {'));
  const role = {};
  for (const m of tagMatch.matchAll(/((?:\s*\|?\s*ElementTag::\w+)+)\s*=>\s*Role::(\w+)/g)) {
    for (const k of m[1].matchAll(/ElementTag::(\w+)/g)) role[k[1]] = m[2];
  }
  const fallback = /_\s*=>\s*Role::(\w+)/.exec(tagMatch)?.[1];
  // Form controls take their role from their control type instead.
  const controlRole = {};
  const controlMatch = roleFn.slice(roleFn.indexOf('match control {'), roleFn.indexOf('match tag {'));
  for (const m of controlMatch.matchAll(/FormControlRole::(\w+)\s*=>\s*Role::(\w+)/g)) controlRole[m[1]] = m[2];

  const interaction = read('bindings/rust/openui-engine/src/interaction.rs');
  const forTag = functionBody(interaction, 'pub(crate) fn for_tag(');
  const control = {};
  for (const m of forTag.matchAll(/ElementTag::(\w+)\s*=>\s*FormControlRole::(\w+)/g)) control[m[1]] = m[2];
  // input's role follows its `type` attribute.
  const typeArm = interaction.slice(interaction.indexOf('"type" if is_input'));
  const typeMatch = typeArm.slice(0, typeArm.indexOf('};'));
  const inputTypes = [];
  for (const m of typeMatch.matchAll(/((?:\s*\|?\s*"[a-z]+")+)\s*=>\s*FormControlRole::(\w+)/g)) {
    for (const t of m[1].matchAll(/"([a-z]+)"/g)) inputTypes.push({ type: t[1], role: controlRole[m[2]] });
  }
  const other = /_\s*=>\s*FormControlRole::(\w+)/.exec(typeMatch)?.[1];
  inputTypes.push({ type: '(any other, or none)', role: controlRole[other] });

  const kebab = (s) => s.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
  return {
    elements: rows.map((row) => ({
      tag: row.tag,
      kind: row.kind,
      display: kebab(row.displayOverride ?? display[row.kind] ?? 'unknown'),
      control: control[row.kind] !== undefined,
      role: control[row.kind] ? controlRole[control[row.kind]] : (role[row.kind] ?? fallback),
    })),
    inputTypes,
  };
}

const outputs = {
  'src/data/elements.json': elements(),
};

const check = process.argv.includes('--check');
let stale = false;
for (const [path, data] of Object.entries(outputs)) {
  const text = JSON.stringify(data, null, 2) + '\n';
  const file = join(site, path);
  let current = '';
  try {
    current = readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
  } catch {}
  if (current === text) continue;
  if (check) {
    console.error(`${path} is out of date: run node scripts/gen-reference.mjs`);
    stale = true;
  } else {
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, text);
    console.log(`wrote ${path}`);
  }
}
if (stale) process.exit(1);
