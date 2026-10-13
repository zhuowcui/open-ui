// Version 1 straight-line native replay inventory. This parses scripts; it
// never evaluates them. Historical mutation audit output remains separate.
import {pathToFileURL} from 'node:url';

const {parse} = await import(pathToFileURL(process.argv[2]).href);
let input = '';
for await (const chunk of process.stdin) input += chunk;

const reads = new Set(['offsetTop', 'offsetLeft', 'offsetWidth', 'offsetHeight',
  'clientTop', 'clientLeft', 'clientWidth', 'clientHeight', 'scrollWidth', 'scrollHeight']);
const queries = new Set(['getElementById', 'querySelector']);
const member = n => n?.type === 'MemberExpression'
  ? (!n.computed && n.property.type === 'Identifier' ? n.property.name
    : n.property.type === 'Literal' ? n.property.value : null) : null;
const literal = n => n?.type === 'Literal' && ['string', 'number'].includes(typeof n.value)
  && (typeof n.value !== 'number' || Number.isFinite(n.value)) ? n.value : undefined;

function analyze(document) {
  const operations = [], rejections = [];
  const declarations = new Set();
  let scriptIndex = 0, origin = '';
  const reject = (n, detail) => rejections.push({reason: 'outside-native-replay-v1',
    script_index: scriptIndex, origin, source_offset: n?.start ?? null, detail});
  const emit = (kind, n, fields = {}) => operations.push({order: operations.length,
    kind, script_index: scriptIndex, origin, source_offset: n.start, ...fields});
  function target(n) {
    if (n?.type === 'Identifier') return {kind: 'binding', name: n.name};
    if (n?.type === 'MemberExpression' && member(n) === 'body'
        && n.object.type === 'Identifier' && n.object.name === 'document') {
      return {kind: 'body'};
    }
    if (n?.type === 'CallExpression' && queries.has(member(n.callee))
        && n.callee.object.type === 'Identifier' && n.callee.object.name === 'document'
        && n.arguments.length === 1 && typeof literal(n.arguments[0]) === 'string') {
      return {kind: 'query', method: member(n.callee), argument: literal(n.arguments[0])};
    }
    reject(n, 'unhandled node target');
    return {kind: 'unsupported'};
  }
  function expression(n, binding = null) {
    if (n.type === 'AssignmentExpression') {
      if (n.operator !== '=' || n.left.type !== 'MemberExpression'
          || member(n.left.object) !== 'style' || typeof member(n.left) !== 'string'
          || literal(n.right) === undefined) {
        reject(n, 'unhandled assignment'); return;
      }
      const value = String(literal(n.right));
      emit(value === '' ? 'remove-style' : 'set-style', n,
        {target: target(n.left.object.object), property: member(n.left), value});
      return;
    }
    if (n.type === 'MemberExpression' && reads.has(member(n))) {
      emit('layout-barrier', n, {target: target(n.object), property: member(n)}); return;
    }
    if (n.type === 'CallExpression') {
      const name = member(n.callee), receiver = n.callee.object;
      if (queries.has(name)) {
        const resolved = target(n);
        if (binding) emit('bind-target', n, {binding, target: resolved});
      } else if (name === 'createElement' && receiver?.type === 'Identifier'
          && receiver.name === 'document' && binding && n.arguments.length === 1
          && ['div', 'span'].includes(literal(n.arguments[0]))) {
        emit('create-element', n, {binding, tag_name: literal(n.arguments[0])});
      } else if (['prepend', 'append', 'appendChild'].includes(name)
          && n.arguments.length && (name !== 'appendChild' || n.arguments.length === 1)
          && n.arguments.every(a => a.type === 'Identifier')) {
        emit(name === 'prepend' ? 'prepend' : 'append', n,
          {target: target(receiver), children: n.arguments.map(target)});
      } else if (name === 'getBoundingClientRect' && n.arguments.length === 0) {
        emit('layout-barrier', n, {target: target(receiver), method: name});
      } else {
        reject(n, `unhandled call ${name ?? n.callee.type}`);
      }
      return;
    }
    // Pure literals have no effect. All other expressions must be modeled.
    if (n.type !== 'Literal') reject(n, `unhandled expression ${n.type}`);
  }
  for (const source of document.external_scripts ?? []) reject(null, `external script ${source}`);
  for (const script of document.scripts) {
    origin = script.origin;
    if (origin.startsWith('inline-handler:')) {
      reject(null, 'event handler ordering is outside this cohort'); scriptIndex++; continue;
    }
    const attrs = script.attrs ?? {};
    if (Object.keys(attrs).some(name => name !== 'type')
        || !['', 'text/javascript', 'application/javascript'].includes(
          String(attrs.type ?? '').trim().toLowerCase())) {
      reject(null, 'unsupported script activation or ordering'); scriptIndex++; continue;
    }
    let ast;
    try { ast = parse(script.code, {ecmaVersion: 'latest', sourceType: 'script'}); }
    catch (error) { reject(null, `parse error: ${error.message}`); scriptIndex++; continue; }
    for (const statement of ast.body) {
      if (statement.type === 'ExpressionStatement') expression(statement.expression);
      else if (statement.type === 'VariableDeclaration') {
        for (const declaration of statement.declarations) {
          if (declaration.id.type !== 'Identifier' || !declaration.init) {
            reject(declaration, 'unhandled declaration'); continue;
          }
          if (['document', 'window'].includes(declaration.id.name)
              || declarations.has(declaration.id.name)) {
            reject(declaration, 'reserved or repeated binding'); continue;
          }
          declarations.add(declaration.id.name);
          expression(declaration.init, declaration.id.name);
        }
      } else if (statement.type !== 'EmptyStatement') {
        reject(statement, `unhandled statement ${statement.type}`);
      }
    }
    scriptIndex++;
  }
  return {schema_version: 1, test_id: document.test_id, operations, rejections,
    lowerable: rejections.length === 0
      && operations.some(o => !['bind-target', 'layout-barrier'].includes(o.kind))};
}
process.stdout.write(JSON.stringify(JSON.parse(input).map(analyze)));
