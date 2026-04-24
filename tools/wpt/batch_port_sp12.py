#!/usr/bin/env python3
"""
Batch port ALL SP12 WPT tests.

Usage:
  python3 tools/wpt/batch_port_sp12.py

Processes all WPT test directories for SP12 scope, generates:
  - Rust module files in bindings/rust/pixel-compare/src/wpt/
  - HTML template JSON files
  - Summary reports
"""

import os
import sys
import json
import csv
from pathlib import Path

# Add project root to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)) + '/../..')
from tools.wpt.port_wpt import process_directory, write_rust_module, write_html_templates, write_report

CHROMIUM_WPT = os.path.expanduser(
    "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
)

OUTPUT_RUST = os.path.join(os.path.dirname(__file__), "../../bindings/rust/pixel-compare/src/wpt")
OUTPUT_DATA = os.path.join(os.path.dirname(__file__), "../accountability/data/wpt_ported")

# SP12 WPT test directories and their module names
SP12_DIRS = [
    ("CSS2/floats", "css2_floats"),
    ("css-position", "css_position"),
    ("css-flexbox", "css_flexbox"),
    ("css-multicol", "css_multicol"),
    ("css-overflow", "css_overflow"),
    ("css-sizing", "css_sizing"),
    ("css-break", "css_break"),
    ("css-display", "css_display"),
    ("css-box", "css_box"),
    ("css-backgrounds", "css_backgrounds"),
]


def find_all_html_files(base_dir: str) -> list:
    """Find all HTML files recursively, returning paths relative to base_dir."""
    result = []
    for root, dirs, files in os.walk(base_dir):
        # Skip support/reference directories
        base = os.path.basename(root)
        if base in ('support', 'reference', 'crashtests', 'tentative'):
            continue
        for f in sorted(files):
            if f.endswith('.html') or f.endswith('.xht') or f.endswith('.xhtml'):
                full = os.path.join(root, f)
                result.append(full)
    return result


def process_recursive(base_dir: str, module_name: str) -> dict:
    """Process a WPT directory recursively."""
    from tools.wpt.port_wpt import (
        parse_wpt_html, analyze_portability, sanitize_fn_name,
        generate_rust_fn, generate_html_template, generate_style_code
    )

    results = {
        'portable': [],
        'not_portable': [],
        'errors': [],
    }

    html_files = find_all_html_files(base_dir)
    print(f"Found {len(html_files)} HTML files in {base_dir} (recursive)")

    for html_path in html_files:
        rel_path = os.path.relpath(html_path, base_dir)
        filename = Path(html_path).stem

        # Create a unique name from the relative path
        unique_name = rel_path.replace('/', '_').replace('\\', '_')
        unique_name = Path(unique_name).stem

        try:
            parser = parse_wpt_html(html_path)
            portable, reason = analyze_portability(parser)

            if not portable:
                results['not_portable'].append((unique_name, reason))
                continue

            # Check if DOM tree has any layout children
            layout_children = []
            for c in parser.root.children:
                if c.is_text:
                    continue
                if c.tag == 'p' and not c.styles:
                    has_test_text = False
                    for sub in c.children:
                        if sub.is_text and 'test passes' in getattr(sub, 'text_content', '').lower():
                            has_test_text = True
                    if has_test_text:
                        continue
                if c.tag == 'br':
                    continue
                layout_children.append(c)

            if not layout_children:
                results['not_portable'].append((unique_name, "no_layout_content"))
                continue

            fn_name = f"{module_name}_{sanitize_fn_name(unique_name)}"
            rust_code = generate_rust_fn(fn_name, parser.root, parser.html_styles)
            html_template = generate_html_template(html_path)

            results['portable'].append((unique_name, fn_name, rust_code, html_template))

        except Exception as e:
            results['errors'].append((unique_name, str(e)))

    return results


def main():
    os.makedirs(OUTPUT_RUST, exist_ok=True)
    os.makedirs(OUTPUT_DATA, exist_ok=True)

    grand_total = 0
    grand_ported = 0
    grand_not_portable = 0
    grand_errors = 0

    all_templates = {}
    all_registry_entries = []
    module_names = []

    for rel_dir, module_name in SP12_DIRS:
        wpt_dir = os.path.join(CHROMIUM_WPT, rel_dir)
        if not os.path.isdir(wpt_dir):
            print(f"SKIP: {wpt_dir} not found")
            continue

        print(f"\n{'='*60}")
        print(f"Processing: {rel_dir} → {module_name}")
        print(f"{'='*60}")

        results = process_recursive(wpt_dir, module_name)

        total = len(results['portable']) + len(results['not_portable']) + len(results['errors'])
        grand_total += total
        grand_ported += len(results['portable'])
        grand_not_portable += len(results['not_portable'])
        grand_errors += len(results['errors'])

        # Write Rust module
        rust_path = os.path.join(OUTPUT_RUST, f"wpt_{module_name}.rs")
        write_rust_module(results, rust_path, module_name)

        # Write HTML templates
        html_path = os.path.join(OUTPUT_DATA, f"wpt_{module_name}_templates.json")
        write_html_templates(results, html_path, module_name)

        # Write report
        report_path = os.path.join(OUTPUT_DATA, f"wpt_{module_name}_report.csv")
        write_report(results, report_path)

        # Collect for combined outputs
        for filename, fn_name, rust_code, html_template in results['portable']:
            test_id = f"wpt/{module_name}/{filename}"
            all_templates[test_id] = html_template
            all_registry_entries.append((test_id, fn_name, module_name))

        module_names.append(module_name)

        # Print breakdown
        if results['not_portable']:
            reasons = {}
            for _, reason in results['not_portable']:
                reasons[reason] = reasons.get(reason, 0) + 1
            print(f"\n  Not-portable breakdown:")
            for reason, count in sorted(reasons.items(), key=lambda x: -x[1])[:10]:
                print(f"    {reason}: {count}")

    # Write combined HTML templates
    combined_path = os.path.join(OUTPUT_DATA, "all_wpt_templates.json")
    with open(combined_path, 'w') as f:
        json.dump(all_templates, f, indent=2)
    print(f"\nWrote combined templates: {combined_path}")

    # Write mod.rs for the wpt module
    mod_path = os.path.join(OUTPUT_RUST, "mod.rs")
    with open(mod_path, 'w') as f:
        f.write("//! WPT test modules — auto-generated\n\n")
        f.write("use openui_dom::Document;\n\n")
        for mn in module_names:
            f.write(f"pub mod wpt_{mn};\n")
        f.write("\n")
        f.write("pub fn all_wpt_registry() -> Vec<(&'static str, fn() -> Document)> {\n")
        f.write("    let mut all = Vec::new();\n")
        for mn in module_names:
            f.write(f"    all.extend(wpt_{mn}::{mn}_registry());\n")
        f.write("    all\n")
        f.write("}\n")
    print(f"Wrote {mod_path}")

    # Grand summary
    print(f"\n{'='*60}")
    print(f"GRAND TOTAL SP12 WPT PORTING SUMMARY")
    print(f"{'='*60}")
    print(f"  Total HTML files scanned: {grand_total}")
    print(f"  Successfully ported:      {grand_ported}")
    print(f"  Not portable:             {grand_not_portable}")
    print(f"  Errors:                   {grand_errors}")
    print(f"  Port rate:                {grand_ported*100/max(grand_total,1):.1f}%")
    print(f"\n  Tests are in: {OUTPUT_RUST}/")
    print(f"  Templates in: {OUTPUT_DATA}/")


if __name__ == '__main__':
    main()
