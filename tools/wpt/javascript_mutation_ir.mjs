#!/usr/bin/env node
// Build an evidence-bearing, ordered mutation inventory from JavaScript ASTs.

import {pathToFileURL} from 'node:url';

const acornPath = process.argv[2];
if (!acornPath) throw new Error('usage: javascript_mutation_ir.mjs ACORN_MJS');
const {parse} = await import(pathToFileURL(acornPath).href);

let input = '';
for await (const chunk of process.stdin) input += chunk;
const documents = JSON.parse(input);

const ASYNC_TIMERS = new Set(['setTimeout', 'setInterval', 'queueMicrotask']);
const ASYNC_FRAMES = new Set(['requestAnimationFrame', 'cancelAnimationFrame']);
const NETWORK = new Set(['fetch', 'XMLHttpRequest', 'WebSocket', 'EventSource']);
const RESOURCE_CONSTRUCTORS = new Set(['Image', 'URL', 'Audio']);
const RESOURCE_PROPERTIES = new Set(['src', 'srcdoc', 'href', 'data']);
const ASSERTIONS = new Set([
  'test', 'promise_test', 'async_test', 'setup', 'promise_setup',
  'assert_true', 'assert_false', 'assert_equals', 'assert_not_equals',
  'assert_array_equals', 'assert_approx_equals', 'assert_unreached',
]);
const LAYOUT_PROPERTIES = new Set([
  'offsetTop', 'offsetLeft', 'offsetWidth', 'offsetHeight',
  'clientTop', 'clientLeft', 'clientWidth', 'clientHeight',
  'scrollWidth', 'scrollHeight',
]);
const TEXT_PROPERTIES = new Set(['textContent', 'innerText', 'nodeValue', 'data']);
const SCROLL_PROPERTIES = new Set(['scrollTop', 'scrollLeft']);

function propertyName(node) {
  if (!node) return null;
  if (!node.computed && node.property?.type === 'Identifier') return node.property.name;
  if (node.computed && node.property?.type === 'Literal'
      && ['string', 'number'].includes(typeof node.property.value)) {
    return node.property.value;
  }
  return null;
}

function calleeName(node) {
  if (!node) return null;
  if (node.type === 'Identifier') return node.name;
  if (node.type === 'MemberExpression') return propertyName(node);
  return null;
}

function constant(node) {
  if (!node) return {known: false};
  if (node.type === 'Literal') return {known: true, value: node.value};
  if (node.type === 'TemplateLiteral' && node.expressions.length === 0) {
    return {known: true, value: node.quasis.map(part => part.value.cooked).join('')};
  }
  if (node.type === 'ArrayExpression') {
    const values = node.elements.map(constant);
    return values.every(value => value.known)
      ? {known: true, value: values.map(value => value.value)} : {known: false};
  }
  if (node.type === 'UnaryExpression') {
    const argument = constant(node.argument);
    if (!argument.known) return {known: false};
    if (node.operator === '!') return {known: true, value: !argument.value};
    if (node.operator === '+') return {known: true, value: +argument.value};
    if (node.operator === '-') return {known: true, value: -argument.value};
    if (node.operator === 'void') return {known: true, value: null};
  }
  return {known: false};
}

function target(node) {
  if (!node) return {kind: 'unknown'};
  if (node.type === 'Identifier') return {kind: 'binding', name: node.name};
  if (node.type === 'ThisExpression') return {kind: 'this'};
  if (node.type === 'MemberExpression') {
    return {kind: 'member', object: target(node.object), property: propertyName(node)};
  }
  if (node.type === 'CallExpression' && node.callee.type === 'MemberExpression') {
    const method = propertyName(node.callee);
    if (['getElementById', 'querySelector', 'querySelectorAll', 'getElementsByClassName',
         'getElementsByTagName'].includes(method)) {
      const argument = constant(node.arguments[0]);
      return {
        kind: 'dom-query',
        root: target(node.callee.object),
        method,
        argument: argument.known ? argument.value : null,
        constant: argument.known,
      };
    }
    if (['createElement', 'createTextNode', 'createDocumentFragment'].includes(method)) {
      return {kind: 'created-node', source_offset: node.start};
    }
    if (method === 'createElementNS') {
      return {kind: 'created-node', source_offset: node.start};
    }
    if (method === 'cloneNode') {
      return {kind: 'created-node', source_offset: node.start};
    }
    if (['append', 'appendChild', 'insertBefore'].includes(method) && node.arguments.length) {
      return target(node.arguments[0]);
    }
  }
  return {kind: 'expression', node_type: node.type};
}

function hasDynamicTarget(value) {
  if (!value || typeof value !== 'object') return false;
  if (value.kind === 'dom-query' && !value.constant) return true;
  return Object.values(value).some(hasDynamicTarget);
}

function assignedBinding(parent, node) {
  if (parent?.type === 'VariableDeclarator' && parent.init === node
      && parent.id.type === 'Identifier') return parent.id.name;
  if (parent?.type === 'AssignmentExpression' && parent.right === node
      && parent.left.type === 'Identifier') return parent.left.name;
  return null;
}

function childNodes(node) {
  const children = [];
  for (const [key, value] of Object.entries(node)) {
    if (key === 'parent' || key === 'loc' || key === 'start' || key === 'end') continue;
    if (Array.isArray(value)) {
      for (const item of value) if (item && typeof item.type === 'string') children.push(item);
    } else if (value && typeof value.type === 'string') {
      children.push(value);
    }
  }
  return children;
}

function staticLoopIterations(node) {
  if (node.type === 'ForOfStatement') {
    const values = constant(node.right);
    return values.known && Array.isArray(values.value) && values.value.length <= 1024
      ? values.value.length : null;
  }
  if (node.type !== 'ForStatement') return null;
  const init = node.init;
  const test = node.test;
  const update = node.update;
  if (init?.type !== 'VariableDeclaration' || init.declarations.length !== 1
      || init.declarations[0].id.type !== 'Identifier'
      || test?.type !== 'BinaryExpression'
      || test.left.type !== 'Identifier'
      || test.left.name !== init.declarations[0].id.name) return null;
  const start = constant(init.declarations[0].init);
  const limit = constant(test.right);
  if (!start.known || !limit.known || typeof start.value !== 'number'
      || typeof limit.value !== 'number') return null;
  let step = null;
  if (update?.type === 'UpdateExpression'
      && update.argument.type === 'Identifier'
      && update.argument.name === test.left.name) {
    step = update.operator === '++' ? 1 : update.operator === '--' ? -1 : null;
  } else if (update?.type === 'AssignmentExpression'
             && update.left.type === 'Identifier'
             && update.left.name === test.left.name
             && ['+=', '-='].includes(update.operator)) {
    const amount = constant(update.right);
    if (amount.known && typeof amount.value === 'number') {
      step = update.operator === '+=' ? amount.value : -amount.value;
    }
  }
  if (!step || !Number.isFinite(step)) return null;
  const compare = value => ({
    '<': value < limit.value,
    '<=': value <= limit.value,
    '>': value > limit.value,
    '>=': value >= limit.value,
  })[test.operator];
  if (compare(start.value) === undefined) return null;
  let value = start.value;
  let iterations = 0;
  while (compare(value)) {
    if (++iterations > 1024) return null;
    value += step;
  }
  return iterations;
}

function analyzeScript(script, scriptIndex, state) {
  let ast;
  try {
    ast = parse(script.code, {
      ecmaVersion: 'latest',
      sourceType: 'script',
      locations: true,
      allowAwaitOutsideFunction: true,
      allowReturnOutsideFunction: true,
    });
  } catch (error) {
    state.rejections.push({
      reason: 'javascript-parse-error',
      origin: script.origin,
      line: error.loc?.line ?? null,
      detail: String(error.message),
    });
    return;
  }

  function reject(reason, node, detail) {
    state.rejections.push({
      reason,
      origin: script.origin,
      line: node.loc?.start.line ?? null,
      node_type: node.type,
      detail,
    });
  }

  function operation(kind, node, fields = {}) {
    if (hasDynamicTarget(fields)) {
      reject('runtime-dependent-query-target', node, kind);
    }
    state.operations.push({
      order: state.operations.length,
      kind,
      origin: script.origin,
      script_index: scriptIndex,
      source_offset: node.start,
      line: node.loc?.start.line ?? null,
      ...fields,
    });
  }

  function requireConstant(node, owner, label) {
    const value = constant(node);
    if (!value.known) reject('runtime-dependent-mutation-input', owner, label);
    return value.known ? value.value : null;
  }

  for (const statement of ast.body) {
    if (statement.type === 'FunctionDeclaration' && statement.id?.name) {
      state.functions.set(statement.id.name, statement);
    }
  }

  function schedule(callback, node, eventName) {
    if (!callback) {
      reject('runtime-dependent-event-handler', node, eventName);
      return;
    }
    if (callback.async || callback.generator) {
      reject('asynchronous-control-flow', callback, eventName);
      return;
    }
    const functionNode = callback.type === 'Identifier'
      ? state.functions.get(callback.name) : callback;
    if (!functionNode?.body) {
      reject('runtime-dependent-event-handler', node, eventName);
      return;
    }
    state.deferred.push(() => visit(functionNode.body, [], functionNode));
  }

  function visit(node, controls = [], parent = null) {
    if (!node) return;
    // Declarations and callback literals are inert until a direct call or a
    // deterministic document/window load handler schedules them. Traversing
    // their bodies here would produce source order, not execution order.
    if (node.type === 'FunctionDeclaration'
        || node.type === 'FunctionExpression'
        || node.type === 'ArrowFunctionExpression') {
      return;
    }
    if (node.type === 'IfStatement') {
      const condition = constant(node.test);
      if (condition.known) {
        visit(condition.value ? node.consequent : node.alternate, controls, node);
      } else {
        reject('runtime-dependent-control-flow', node, node.type);
        const dynamic = controls.concat([{kind: node.type, static: false}]);
        visit(node.consequent, dynamic, node);
        visit(node.alternate, dynamic, node);
      }
      return;
    }
    if (node.type === 'ConditionalExpression') {
      const condition = constant(node.test);
      if (condition.known) {
        visit(condition.value ? node.consequent : node.alternate, controls, node);
      } else {
        reject('runtime-dependent-control-flow', node, node.type);
        const dynamic = controls.concat([{kind: node.type, static: false}]);
        visit(node.consequent, dynamic, node);
        visit(node.alternate, dynamic, node);
      }
      return;
    }
    if (node.type === 'SwitchStatement') {
      const discriminant = constant(node.discriminant);
      if (!discriminant.known) {
        reject('runtime-dependent-control-flow', node, node.type);
        const dynamic = controls.concat([{kind: node.type, static: false}]);
        for (const branch of node.cases) {
          for (const statement of branch.consequent) visit(statement, dynamic, branch);
        }
        return;
      }
      let selected = node.cases.findIndex(branch => {
        const value = constant(branch.test);
        return value.known && value.value === discriminant.value;
      });
      if (selected < 0) selected = node.cases.findIndex(branch => branch.test === null);
      for (let index = selected; index >= 0 && index < node.cases.length; index += 1) {
        let stopped = false;
        for (const statement of node.cases[index].consequent) {
          if (statement.type === 'BreakStatement') {
            stopped = true;
            break;
          }
          visit(statement, controls.concat([{kind: node.type, static: true}]), node.cases[index]);
        }
        if (stopped) break;
      }
      return;
    }
    if (['ForStatement', 'ForOfStatement', 'ForInStatement',
         'WhileStatement', 'DoWhileStatement'].includes(node.type)) {
      const iterations = staticLoopIterations(node);
      if (iterations === null) {
        reject('unbounded-or-runtime-loop', node, node.type);
        visit(node.body, controls.concat([{kind: node.type, static: false}]), node);
      } else {
        for (let index = 0; index < iterations; index += 1) {
          visit(node.body, controls.concat([{kind: node.type, static: true}]), node);
        }
      }
      return;
    }
    if (node.type === 'VariableDeclarator' && node.id.type === 'Identifier') {
      const value = target(node.init);
      if (['dom-query', 'member'].includes(value.kind)) {
        operation('bind-target', node, {binding: node.id.name, target: value});
      }
    }
    const dynamicControl = controls.find(control => !control.static);
    if (node.type === 'AwaitExpression' || node.type === 'YieldExpression') {
      reject('asynchronous-control-flow', node, node.type);
    }
    if (node.type === 'NewExpression') {
      const name = calleeName(node.callee);
      if (NETWORK.has(name)) reject('network-dependency', node, name);
      if (RESOURCE_CONSTRUCTORS.has(name)) reject('network-dependency', node, name);
      if (name === 'Promise') reject('promise-dependency', node, name);
    }
    if (node.type === 'MemberExpression') {
      const name = propertyName(node);
      if (name === 'localStorage' || name === 'sessionStorage') {
        reject('storage-dependency', node, name);
      }
      if (LAYOUT_PROPERTIES.has(name)) {
        operation('layout-barrier', node, {target: target(node.object), property: name});
      }
    }
    if (node.type === 'AssignmentExpression' && node.left.type === 'Identifier'
        && node.left.name === 'onload') {
      schedule(node.right, node, 'window.onload');
    } else if (node.type === 'AssignmentExpression' && node.left.type === 'Identifier') {
      const value = target(node.right);
      if (['dom-query', 'member'].includes(value.kind)) {
        operation('bind-target', node, {binding: node.left.name, target: value});
      }
    }
    if (node.type === 'AssignmentExpression' && node.left.type === 'MemberExpression') {
      const name = propertyName(node.left);
      const object = node.left.object;
      let mutation = null;
      if (object.type === 'MemberExpression' && propertyName(object) === 'style') {
        mutation = name === 'cssText'
          ? ['set-style-text', {target: target(object.object), value: requireConstant(node.right, node, 'style text')}]
          : ['set-style', {target: target(object.object), property: name, value: requireConstant(node.right, node, 'style value')}];
      } else if (name === 'className') {
        mutation = ['set-attribute', {target: target(object), name: 'class', value: requireConstant(node.right, node, 'class name')}];
      } else if (['id', 'type', 'value', 'open'].includes(name)) {
        mutation = ['set-attribute', {target: target(object), name, value: requireConstant(node.right, node, `reflected ${name} attribute`)}];
      } else if (TEXT_PROPERTIES.has(name)) {
        mutation = ['replace-text', {target: target(object), property: name, value: requireConstant(node.right, node, 'text')}];
      } else if (name === 'innerHTML') {
        const markup = requireConstant(node.right, node, 'static markup');
        if (typeof markup === 'string' && /<(?:iframe|img|link|object|script|source|video)\b/i.test(markup)) {
          reject('network-dependency', node, 'resource-bearing static markup');
        }
        mutation = ['replace-children-markup', {target: target(object), markup}];
      } else if (SCROLL_PROPERTIES.has(name)) {
        mutation = ['scroll-axis', {target: target(object), axis: name, value: requireConstant(node.right, node, 'scroll offset')}];
      } else if (name === 'onload') {
        const receiver = target(object);
        if (receiver.kind === 'binding' && receiver.name === 'window') {
          schedule(node.right, node, 'window.onload');
        } else {
          reject('event-driven-control-flow', node, 'element.onload');
        }
      } else if (name === 'location' || (target(object).name === 'location' && name === 'href')) {
        reject('navigation-dependency', node, name);
      } else if (RESOURCE_PROPERTIES.has(name)) {
        reject('network-dependency', node, name);
      }
      if (mutation) {
        if (object.type === 'CallExpression') visit(object, controls, node.left);
        if (dynamicControl) reject('runtime-dependent-control-flow', node, dynamicControl.kind);
        operation(mutation[0], node, mutation[1]);
        visit(node.right, controls, node);
        return;
      }
    }
    if (node.type === 'CallExpression') {
      // JavaScript evaluates the receiver and arguments before applying the
      // call itself. Callback literals stay inert until explicitly invoked or
      // scheduled below.
      if (node.callee.type === 'MemberExpression') {
        visit(node.callee.object, controls, node.callee);
        if (node.callee.computed) visit(node.callee.property, controls, node.callee);
      }
      for (const argument of node.arguments) visit(argument, controls, node);
      const name = calleeName(node.callee);
      if (ASYNC_TIMERS.has(name)) reject('timer-dependency', node, name);
      if (ASYNC_FRAMES.has(name)) reject('animation-frame-dependency', node, name);
      if (NETWORK.has(name)) reject('network-dependency', node, name);
      if (name === 'random' && target(node.callee.object).name === 'Math') {
        reject('runtime-dependent-mutation-input', node, 'Math.random');
      }
      if (ASSERTIONS.has(name) || (name?.startsWith('assert_') ?? false)) {
        reject('assertion-harness-dependency', node, name);
      }
      if (['then', 'catch', 'finally'].includes(name)) reject('promise-dependency', node, name);
      if (['click', 'dispatchEvent'].includes(name)) reject('event-synthesis-dependency', node, name);
      if (name === 'getContext') reject('canvas-dependency', node, name);
      if (['assign', 'replace', 'reload', 'open'].includes(name)
          && ['location', 'window'].includes(target(node.callee.object).name)) {
        reject('navigation-dependency', node, name);
      }

      const object = node.callee.type === 'MemberExpression' ? node.callee.object : null;
      let mutation = null;
      if (name === 'createElement') {
        const binding = assignedBinding(parent, node);
        const tagName = requireConstant(node.arguments[0], node, 'element name');
        if (typeof tagName === 'string'
            && ['iframe', 'img', 'link', 'object', 'script', 'source', 'video'].includes(tagName.toLowerCase())) {
          reject('network-dependency', node, `createElement(${tagName})`);
        }
        mutation = ['create-element', {binding, tag_name: tagName}];
      } else if (name === 'createElementNS') {
        const binding = assignedBinding(parent, node);
        const namespace = requireConstant(node.arguments[0], node, 'element namespace');
        const tagName = requireConstant(node.arguments[1], node, 'element name');
        if (namespace !== 'http://www.w3.org/1999/xhtml') {
          reject('unsupported-element-namespace', node, String(namespace));
        }
        mutation = ['create-element', {binding, tag_name: tagName, namespace}];
      } else if (name === 'createTextNode') {
        const binding = assignedBinding(parent, node);
        mutation = ['create-text', {binding, text: requireConstant(node.arguments[0], node, 'text')}];
      } else if (name === 'createDocumentFragment') {
        const binding = assignedBinding(parent, node);
        mutation = ['create-fragment', {binding}];
      } else if (name === 'cloneNode') {
        const binding = assignedBinding(parent, node);
        mutation = ['clone-target', {
          binding,
          target: target(object),
          deep: Boolean(requireConstant(node.arguments[0], node, 'clone depth')),
        }];
      } else if (['append', 'appendChild'].includes(name)) {
        mutation = ['append', {target: target(object), children: node.arguments.map(target)}];
      } else if (name === 'insertBefore') {
        mutation = ['insert-before', {target: target(object), child: target(node.arguments[0]), before: target(node.arguments[1])}];
      } else if (object?.type === 'MemberExpression' && propertyName(object) === 'classList'
                 && ['add', 'remove', 'toggle', 'replace'].includes(name)) {
        mutation = ['class-list', {target: target(object.object), action: name, tokens: node.arguments.map(argument => requireConstant(argument, node, 'class token'))}];
      } else if (name === 'remove') {
        mutation = ['remove', {target: target(object)}];
      } else if (name === 'removeChild') {
        mutation = ['remove', {target: target(node.arguments[0]), parent: target(object)}];
      } else if (name === 'setAttribute') {
        const attributeName = requireConstant(node.arguments[0], node, 'attribute name');
        if (typeof attributeName === 'string'
            && RESOURCE_PROPERTIES.has(attributeName.toLowerCase())) {
          reject('network-dependency', node, `setAttribute(${attributeName})`);
        }
        mutation = ['set-attribute', {target: target(object), name: attributeName, value: requireConstant(node.arguments[1], node, 'attribute value')}];
      } else if (name === 'removeAttribute') {
        mutation = ['remove-attribute', {target: target(object), name: requireConstant(node.arguments[0], node, 'attribute name')}];
      } else if (object?.type === 'MemberExpression' && propertyName(object) === 'style'
                 && name === 'setProperty') {
        mutation = ['set-style', {target: target(object.object), property: requireConstant(node.arguments[0], node, 'style property'), value: requireConstant(node.arguments[1], node, 'style value')}];
      } else if (['scroll', 'scrollTo'].includes(name)) {
        mutation = ['scroll-to', {target: target(object), arguments: node.arguments.map(argument => requireConstant(argument, node, 'scroll argument'))}];
      } else if (name === 'getBoundingClientRect' || name === 'getComputedStyle') {
        operation('layout-barrier', node, {target: target(object), method: name});
      } else if (name === 'addEventListener') {
        const eventName = requireConstant(node.arguments[0], node, 'event name');
        const receiver = target(object);
        if ((eventName === 'load' || eventName === 'DOMContentLoaded')
            && receiver.kind === 'binding'
            && ['window', 'document'].includes(receiver.name)) {
          schedule(node.arguments[1], node, eventName);
        } else {
          reject('event-driven-control-flow', node, String(eventName));
        }
      }
      if (mutation) {
        if (dynamicControl) reject('runtime-dependent-control-flow', node, dynamicControl.kind);
        operation(mutation[0], node, mutation[1]);
      }
      if (node.callee.type === 'Identifier' && state.functions.has(node.callee.name)) {
        const functionNode = state.functions.get(node.callee.name);
        if (functionNode.async || functionNode.generator) {
          reject('asynchronous-control-flow', node, node.callee.name);
        } else if (state.activeFunctions.has(node.callee.name)) {
          reject('recursive-control-flow', node, node.callee.name);
        } else {
          state.activeFunctions.add(node.callee.name);
          visit(functionNode.body, controls, functionNode);
          state.activeFunctions.delete(node.callee.name);
        }
      } else if (node.callee.type === 'FunctionExpression'
                 || node.callee.type === 'ArrowFunctionExpression') {
        if (node.callee.async || node.callee.generator) {
          reject('asynchronous-control-flow', node, 'immediately invoked function');
        } else {
          visit(node.callee.body, controls, node.callee);
        }
      }
      return;
    }

    for (const child of childNodes(node)) {
      if (child) visit(child, controls, node);
    }
  }
  const execute = () => visit(ast);
  if (script.origin.startsWith('inline-handler:')) {
    const eventName = script.origin.slice('inline-handler:'.length);
    if (eventName === 'onload') state.deferred.push(execute);
    else reject('event-driven-control-flow', ast, eventName);
  } else {
    execute();
  }
}

const results = documents.map(document => {
  const state = {
    operations: [], rejections: [], functions: new Map(),
    deferred: [], activeFunctions: new Set(),
  };
  if (document.external_scripts?.length) {
    for (const source of document.external_scripts) {
      if (!source.startsWith('/common/')) {
        state.rejections.push({
          reason: 'external-script-dependency', origin: source, line: null,
          node_type: 'ScriptElement', detail: source,
        });
      }
    }
  }
  document.scripts.forEach((script, index) => analyzeScript(script, index, state));
  let deferredCount = 0;
  while (state.deferred.length) {
    if (++deferredCount > 1024) {
      state.rejections.push({
        reason: 'unbounded-event-handler-chain', origin: 'document', line: null,
        node_type: 'Program', detail: 'more than 1024 deterministic handlers',
      });
      break;
    }
    state.deferred.shift()();
  }
  const reasons = [...new Set(state.rejections.map(item => item.reason))].sort();
  return {
    test_id: document.test_id,
    ast_status: state.rejections.some(item => item.reason === 'javascript-parse-error') ? 'parse-error' : 'parsed',
    lowerable: state.operations.some(item => !['layout-barrier'].includes(item.kind)) && reasons.length === 0,
    operations: state.operations,
    rejections: state.rejections,
    rejection_reasons: reasons,
  };
});
process.stdout.write(JSON.stringify(results));
