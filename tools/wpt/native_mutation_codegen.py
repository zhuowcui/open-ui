#!/usr/bin/env python3
"""Compile a bounded offline AST inventory to ordered public Engine calls.

This version accepts straight-line create/bind/style/append/prepend/layout
operations. It keeps authored CSS literals, not computed snapshots. Existing
corpus builders, HTML, reference images and historical audit files are inputs.
The generated IDs are pending until Root measures all four required profiles.
"""
from __future__ import annotations

import argparse
import copy
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

try:
    from tools.wpt import port_wpt as porter
except ModuleNotFoundError:
    import port_wpt as porter

ROOT = Path(__file__).resolve().parents[2]
FRONTEND = Path(__file__).with_name("javascript_native_mutation_ir.mjs")
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
ACORN = Path(os.environ.get("CHROMIUM_ACORN", str(Path.home()
    / "chromium/src/third_party/node/node_modules/acorn/dist/acorn.mjs")))
HARNESS = porter.parse_simple_css_rules(
    "* { margin: 0; padding: 0; box-sizing: content-box; } "
    "body { margin: 0; padding: 20px; font-family: DejaVu Sans, sans-serif; "
    "font-size: 16px; color: black; background-color: white; }")
TAGS = {"body": "Body", "div": "Div", "span": "Span"}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def css_name(name: str) -> str:
    return "float" if name == "cssFloat" else re.sub(
        r"[A-Z]", lambda m: "-" + m[0].lower(), name)


def rust_string(value: str) -> str:
    escapes = {"\\": "\\\\", '"': '\\"', "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + ''.join(escapes.get(c, f"\\u{{{ord(c):x}}}" if ord(c) < 32 else c)
                         for c in value) + '"'


def walk(node):
    yield node
    for child in node.children:
        yield from walk(child)


def inventory(source: str, test_id: str, acorn: Path = ACORN) -> dict:
    parser = porter.WptHtmlParser(root_aware=True)
    parser.feed(source)
    scripts = [{"origin": f"inline:{i}", "code": item["content"], "attrs": item["attrs"]}
               for i, item in enumerate(parser.script_elements)]
    scripts += [{"origin": f"inline-handler:{name}", "code": value}
                for _tag, name, value in parser.event_handlers]
    external = [item["attrs"]["src"] for item in parser.script_elements
                if item["attrs"].get("src")]
    document = {"test_id": test_id, "scripts": scripts, "external_scripts": external}
    result = subprocess.run(["node", str(FRONTEND), str(acorn)],
                            input=json.dumps([document]), capture_output=True,
                            text=True, check=True)
    return json.loads(result.stdout)[0]


class ReplayCompiler:
    def __init__(self, source: str, ir: dict):
        if ir.get("schema_version") != 1 or not ir.get("lowerable") or ir.get("rejections"):
            raise ValueError("native replay requires a rejection-free version 1 inventory")
        self.ir = copy.deepcopy(ir)
        self.parser = porter.WptHtmlParser(root_aware=True)
        self.parser.feed(source)
        for script in self.parser.script_elements:
            attrs = script['attrs']
            if set(attrs) - {'type'} or str(attrs.get('type') or '').strip().lower() not in {
                    '', 'text/javascript', 'application/javascript'}:
                raise ValueError('unsupported script activation or ordering')
        if self.parser.external_stylesheet_hrefs or self.parser.keyframes:
            raise ValueError("external stylesheets/animations are outside native replay v1")
        if self.parser.html_styles:
            raise ValueError("authored html declarations are outside native replay v1")
        if any('@' in text for _attrs, text, _prefix in self.parser.author_style_blocks):
            raise ValueError("CSS at-rules are outside native replay v1")
        for _attrs, text, _prefix in self.parser.author_style_blocks:
            if _attrs:
                raise ValueError("conditional/style attributes are outside native replay v1")
            text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
            rules = list(re.finditer(r'([^{}]+)\{([^{}]*)\}', text))
            if re.sub(r'([^{}]+)\{([^{}]*)\}', '', text).strip():
                raise ValueError("unhandled stylesheet syntax")
            for rule in rules:
                if not re.fullmatch(r'[\w.#* >,\s-]+', rule[1]):
                    raise ValueError("unhandled authored selector")
        self.rules = HARNESS + self.parser.css_rules
        for selector, _styles in self.rules:
            if not re.fullmatch(r"[\w.#* >-]+", selector):
                raise ValueError(f"selector outside native replay v1: {selector}")
        # Match the existing legacy box-only capture policy. No source file is
        # rewritten: instructional pass paragraphs and visible text are
        # excluded by that capture environment, as in the historical porter.
        def legacy(node):
            node.children = [c for c in node.children if not c.is_text
                             and not (c.tag == "p" and "test passes" in
                                      porter._subtree_text(c).lower())]
            for child in node.children:
                legacy(child)
        legacy(self.parser.root)
        # The native schema exposes the color longhand. Color-only background
        # shorthand is lossless in this image-free cohort. Refuse other
        # background declarations rather than discard their reset semantics.
        declarations = [styles for _selector, styles in self.rules]
        declarations += [node.styles for node in walk(self.parser.root)]
        for styles in declarations:
            if any(name.startswith('scrollbar-') for name in styles):
                raise ValueError("authored scrollbar precedence is outside native replay v1")
            if any(name.startswith(('animation', 'transition', 'scroll-timeline', 'view-timeline'))
                   or name in {'content', 'counter-reset', 'counter-increment'} for name in styles):
                raise ValueError("effect/generated-content lifecycle is outside native replay v1")
            if any(name.startswith('background-') and name != 'background-color'
                   for name in styles):
                raise ValueError("background image/layer ownership is outside native replay v1")
            if 'background' in styles:
                entries = list(styles.items())
                priorities = dict(getattr(styles, 'cascade_priority', {}))
                important = dict(getattr(styles, 'important', {}))
                styles.clear()
                if isinstance(styles, porter.CssDeclarations):
                    styles.cascade_priority.clear()
                    styles.important.clear()
                for name, value in entries:
                    original_name = name
                    if name == 'background':
                        if porter.parse_color(value) is None and value.lower() != 'currentcolor':
                            raise ValueError("only color-only background shorthand is supported")
                        name = 'background-color'
                    porter._set_declaration(styles, name, value,
                        priority=priorities.get(original_name),
                        important=important.get(original_name, False))
        self.nodes = []
        self.bindings = {}
        self.created = set()
        self.styles = {}
        self.lines = []
        self.operations = []
        self.properties = {row["css_name"]: row["rust_name"]
                           for row in csv.DictReader(SCHEMA.read_text().splitlines())}
        for node in walk(self.parser.root):
            if set(node.attrs) - {'id', 'class', 'style'}:
                raise ValueError("unhandled authored element attribute")
            self.register(node)
        ids = [n.attrs["id"] for n in self.nodes if "id" in n.attrs]
        if len(ids) != len(set(ids)):
            raise ValueError("duplicate DOM IDs are outside native replay v1")

    def register(self, node):
        if node.tag not in TAGS:
            raise ValueError(f"element role outside native replay v1: {node.tag}")
        node.native_replay_key = len(self.nodes)
        self.nodes.append(node)
        return node

    @staticmethod
    def var(node):
        return f"node_{node.native_replay_key}"

    def resolve(self, description):
        kind = description.get("kind")
        if kind == "body":
            return self.parser.root
        if kind == "binding":
            node = self.bindings.get(description.get("name"))
            if node is None:
                raise ValueError("unresolved binding")
            return node
        if kind == "query":
            arg = description.get("argument")
            if description.get("method") == "getElementById":
                found = [n for n in walk(self.parser.root) if n.attrs.get("id") == arg]
            elif description.get("method") == "querySelector" and re.fullmatch(
                    r"[.#]?[\w-]+", arg or ""):
                found = [n for n in walk(self.parser.root) if
                         (n.attrs.get("id") == arg[1:] if arg.startswith("#") else
                          arg[1:] in n.attrs.get("class", "").split() if arg.startswith(".")
                          else n.tag == arg)]
            else:
                raise ValueError("unhandled query selector")
            if not found:
                raise ValueError("query has no retained target")
            return found[0]
        raise ValueError("unhandled target")

    def declaration(self, node, name, value):
        if name not in self.properties or not isinstance(value, str) or not value:
            raise ValueError(f"declaration outside native schema: {name}={value!r}")
        return (f"set_literal(engine, {self.var(node)}, "
                f"StyleProperty::{self.properties[name]}, {rust_string(value)})?;")

    def cascade(self):
        root = copy.deepcopy(self.parser.root)
        porter.apply_css_rules(self.rules, root, ancestors=[("html", [], "", {})])
        result = {}
        for node in walk(root):
            if any(node.pseudo_styles.values()):
                raise ValueError("generated pseudo lifecycle is outside native replay v1")
            priorities = getattr(node.styles, "cascade_priority", {})
            ordered = sorted(node.styles, key=lambda name: priorities.get(name, ()))
            result[node.native_replay_key] = {name: node.styles[name] for name in ordered}
        return result

    def publish_cascade(self):
        current = self.cascade()
        for node in self.nodes:
            key = node.native_replay_key
            if key not in current:
                continue
            old = self.styles.get(key, {})
            new = current[key]
            if tuple(old.items()) != tuple(new.items()):
                # Reapply the surviving raw declarations in cascade order.
                # Engine owns canonical shorthand overlap, units and effects.
                for name in sorted(old.keys() - new.keys()):
                    self.lines.append(f"engine.remove_property({self.var(node)}, "
                                      f"StyleProperty::{self.properties[name]})?;")
                for name, value in new.items():
                    self.lines.append(self.declaration(node, name, value))
            self.styles[key] = new

    def compile(self, function: str) -> tuple[str, dict]:
        if not re.fullmatch(r"[a-z][a-z0-9_]*", function):
            raise ValueError("invalid generated function name")
        self.lines += ["let html = create_capture_node(engine, ElementTag::Html)?;",
                       "engine.append_child(engine.root(), html)?;",
                       "set_literal(engine, html, StyleProperty::Overflow, \"hidden\")?;"]
        for node in self.nodes:
            self.lines.append(f"let {self.var(node)} = create_capture_node(engine, ElementTag::{TAGS[node.tag]})?;")
            for name, value in node.attrs.items():
                if name in {"id", "class"}:
                    self.lines.append(f"engine.set_attribute({self.var(node)}, {rust_string(name)}, {rust_string(value)})?;")
        for parent in self.nodes:
            for child in parent.children:
                self.lines.append(f"engine.append_child({self.var(parent)}, {self.var(child)})?;")
        self.lines.append(f"engine.append_child(html, {self.var(self.parser.root)})?;")
        self.publish_cascade()
        self.lines.append("let mut barriers = Vec::new();")
        for order, operation in enumerate(self.ir["operations"]):
            if operation.get("order") != order:
                raise ValueError("operation order is not contiguous")
            kind = operation["kind"]
            self.lines.append(f"// AST operation {order}: {kind}")
            if kind == "bind-target":
                if operation["binding"] in self.bindings:
                    raise ValueError("binding reassignment is outside native replay v1")
                self.bindings[operation["binding"]] = self.resolve(operation["target"])
            elif kind == "create-element":
                operation['engine_api'] = 'Engine::create_native_element'
                binding = operation["binding"]
                if binding in self.bindings:
                    raise ValueError("binding reassignment is outside native replay v1")
                node = self.register(porter.DomNode(operation["tag_name"], {}, porter.CssDeclarations()))
                self.bindings[binding] = node
                self.created.add(node.native_replay_key)
                self.lines.append(f"let {self.var(node)} = create_capture_node(engine, ElementTag::{TAGS[node.tag]})?;")
            elif kind == "layout-barrier":
                operation['engine_api'] = 'Engine::update'
                self.resolve(operation["target"])
                self.lines.append("let _ = engine.update()?;")
                self.lines.append(f"barriers.push(({order}, engine.stats()));")
            elif kind in {"set-style", "remove-style"}:
                node = self.resolve(operation["target"])
                name = css_name(operation["property"])
                if name not in self.properties:
                    raise ValueError(f"style property outside native schema: {name}")
                if name.startswith(('animation', 'transition', 'scroll-timeline', 'view-timeline')):
                    raise ValueError("effect lifecycle is outside native replay v1")
                if name.startswith('scrollbar-'):
                    raise ValueError("authored scrollbar precedence is outside native replay v1")
                if kind == "remove-style":
                    operation['engine_api'] = 'Engine::remove_property'
                    node.styles.pop(name, None)
                    node.styles.cascade_priority.pop(name, None)
                    node.styles.important.pop(name, None)
                    self.lines.append(f"engine.remove_property({self.var(node)}, StyleProperty::{self.properties[name]})?;")
                    self.styles.get(node.native_replay_key, {}).pop(name, None)
                else:
                    operation['engine_api'] = 'Engine::set_property'
                    index = node.styles.cascade_priority.get(
                        name, (0, 1, 0, 0, 0, 0, len(node.styles)))[-1]
                    # CSSOM assignment replaces an inline value and clears its
                    # priority; it does not inherit an old !important marker.
                    node.styles.cascade_priority.pop(name, None)
                    porter._set_declaration(node.styles, name, operation["value"],
                        priority=(0, 1, 0, 0, 0, 0, index), important=False)
                    # The assignment happens even when the node is detached.
                    self.lines.append(self.declaration(node, name, operation["value"]))
                    # A stronger stylesheet winner can leave the cascade map
                    # unchanged. Re-publish that target so the raw native
                    # setter does not become the effective winner by accident.
                    self.styles.pop(node.native_replay_key, None)
                self.publish_cascade()
                operation['canonical_property'] = {'css_name': name, 'rust_name': self.properties[name]}
            elif kind in {"append", "prepend"}:
                operation['engine_api'] = (['Engine::children', 'Engine::insert_before',
                                           'Engine::append_or_move_child'] if kind == 'prepend'
                                          else ['Engine::append_or_move_child'])
                parent = self.resolve(operation["target"])
                children = [self.resolve(c) for c in operation["children"]]
                if len(set(map(id, children))) != len(children):
                    raise ValueError("duplicate variadic children are outside native replay v1")
                for child in children:
                    if parent in walk(child):
                        raise ValueError("cyclic insertion")
                for child in children:
                    for candidate in self.nodes:
                        if child in candidate.children:
                            candidate.children.remove(child)
                parent.children = children + parent.children if kind == "prepend" else parent.children + children
                # Capture the first child after excluding nodes being moved;
                # insert the arguments in authored order before that anchor.
                if kind == "prepend":
                    self.lines.append(f"let before_{order} = engine.children({self.var(parent)})?.into_iter().find(|node| ![{', '.join(self.var(c) for c in children)}].contains(node));")
                for child in children:
                    if kind == "prepend":
                        self.lines.append(f"if let Some(before) = before_{order} {{ engine.insert_before({self.var(parent)}, {self.var(child)}, before)?; }} else {{ engine.append_or_move_child({self.var(parent)}, {self.var(child)})?; }}")
                    else:
                        self.lines.append(f"engine.append_or_move_child({self.var(parent)}, {self.var(child)})?;")
                self.publish_cascade()
            else:
                raise ValueError(f"unhandled native replay operation: {kind}")
            self.operations.append(copy.deepcopy(operation))
        self.lines.append("let mut handles = std::collections::BTreeMap::new();")
        for node in self.nodes:
            if node.attrs.get("id"):
                self.lines.append(f"handles.insert({rust_string(node.attrs['id'])}.to_string(), {self.var(node)});")
        for name, node in self.bindings.items():
            self.lines.append(f"handles.insert({rust_string('binding:' + name)}.to_string(), {self.var(node)});")
        self.lines.append("Ok(ReplayResult { handles, barriers })")
        code = (f"pub fn {function}(engine: &mut Engine) -> Result<ReplayResult, EngineError> {{\n"
                + "\n".join("    " + line for line in self.lines) + "\n}\n")
        return code, {"schema_version": 1, "test_id": self.ir["test_id"],
                      "native_id": "pending/native-mutation-v1/" + self.ir["test_id"],
                      "function": function, "ir": self.ir, "qualification": False}


HEADER = '''// Generated pending native replay v1. No JavaScript runs in Engine.
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, LifecycleStats, NodeHandle};
use openui_style::{parse_literal, StyleProperty};

pub struct ReplayResult {
    pub handles: std::collections::BTreeMap<String, NodeHandle>,
    pub barriers: Vec<(usize, LifecycleStats)>,
}

fn create_capture_node(engine: &mut Engine, tag: ElementTag) -> Result<NodeHandle, EngineError> {
    let node = engine.create_native_element(tag)?;
    // The immutable browser harness hides WebKit scrollbar pseudo-elements.
    // Authored standard scrollbar precedence is rejected by this cohort.
    set_literal(engine, node, StyleProperty::ScrollbarWidth, "none")?;
    Ok(node)
}

fn set_literal(engine: &mut Engine, node: NodeHandle, property: StyleProperty, literal: &str) -> Result<(), EngineError> {
    let value = parse_literal(property, literal).map_err(|_| EngineError::InvalidInput("unsupported authored CSS literal"))?;
    engine.set_property(node, property, value)
}

'''


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--fixture", nargs=2, action="append", metavar=("TEST_ID", "HTML"), required=True)
    ap.add_argument("--rust", type=Path, required=True)
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--check", action='store_true', help='Compare outputs without writing them')
    args = ap.parse_args()
    code, entries = HEADER, []
    for index, (test_id, path) in enumerate(args.fixture):
        source_path = Path(path)
        ir = inventory(source_path.read_text(encoding="utf-8-sig"), test_id)
        generated, entry = ReplayCompiler(source_path.read_text(encoding="utf-8-sig"), ir).compile(f"replay_{index}")
        entry.update({"source_path": str(source_path.resolve()), "source_sha256": sha256(source_path)})
        parser = porter.WptHtmlParser()
        parser.feed(source_path.read_text(encoding='utf-8-sig'))
        if not parser.ref_path:
            raise ValueError('pending fixture has no original matching reference')
        reference = source_path.parent / parser.ref_path
        entry['reference_path'] = str(reference.resolve())
        entry['reference_sha256'] = sha256(reference)
        code += generated + "\n"
        entries.append(entry)
    # Generated Rust follows the same formatter as hand-authored consumers.
    # No rustc/Cargo execution is involved in this offline source operation.
    formatted = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"],
                               input=code, capture_output=True, text=True, check=True)
    manifest = json.dumps({"schema_version": 1, "frontend_sha256": sha256(FRONTEND),
        'generator_sha256': sha256(Path(__file__)), 'acorn_sha256': sha256(ACORN),
        "schema_sha256": sha256(SCHEMA), "entries": entries, "qualification": False,
        'generated_rust_sha256': hashlib.sha256(formatted.stdout.encode()).hexdigest(),
        'capture_policy': 'existing legacy-box-only harness; raw authored CSS and ordered native calls',
        "required_gate": "Root clean source; all four current-Chromium profiles exact"}, indent=2) + "\n"
    for path, text in [(args.rust, formatted.stdout), (args.manifest, manifest)]:
        if args.check:
            if not path.is_file() or path.read_text() != text:
                raise ValueError(f'generated pending replay output differs: {path}')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)


if __name__ == "__main__":
    main()
