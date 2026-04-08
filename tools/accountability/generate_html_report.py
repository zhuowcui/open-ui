#!/usr/bin/env python3
"""Generate a self-contained HTML dashboard report for Open UI WPT test tracking."""

import csv
import html
import json
import os
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")

SUMMARY_JSON = os.path.join(DATA_DIR, "pixel_comparison", "results", "summary.json")
TEMPLATES_JSON = os.path.join(DATA_DIR, "wpt_ported", "all_wpt_templates.json")
MAPPING_CSV = os.path.join(DATA_DIR, "wpt_mapping.csv")
SP12_CSV = os.path.join(DATA_DIR, "sp12_5_deferred.csv")
OUTPUT_HTML = os.path.join(DATA_DIR, "report.html")

TOTAL_CHROMIUM_WPT = 7673  # fallback if mapping CSV unavailable


def load_summary():
    with open(SUMMARY_JSON) as f:
        return json.load(f)


def load_mapping_csv():
    """Load mapping CSV if it exists. Returns list of dicts or None."""
    if not os.path.isfile(MAPPING_CSV):
        return None
    rows = []
    with open(MAPPING_CSV, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
    return rows


def load_sp12_csv():
    """Load SP12.5 deferred CSV if it exists. Returns list of dicts or None."""
    if not os.path.isfile(SP12_CSV):
        return None
    rows = []
    with open(SP12_CSV, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
    return rows


def extract_area(test_id):
    """Extract area name from test ID like 'wpt/css_flexbox/test-name'."""
    parts = test_id.split("/")
    return parts[1] if len(parts) >= 2 else "unknown"


def build_report_data():
    """Build all data structures needed for the report."""
    summary = load_summary()
    mapping = load_mapping_csv()
    sp12 = load_sp12_csv()

    total_ported = summary["total"]
    total_passed = summary["passed"]
    total_failed = summary["failed"]

    # Build per-test lookup from summary
    test_lookup = {}
    for t in summary["tests"]:
        test_lookup[t["id"]] = t

    # Determine total Chromium tests and per-area Chromium counts
    chromium_total = TOTAL_CHROMIUM_WPT
    chromium_area_counts = {}
    mapping_lookup = {}
    failure_categories = {}

    if mapping:
        # Count all rows in mapping as total Chromium tests
        chromium_total = len(mapping)
        for row in mapping:
            area = row.get("sp_area", "unknown").strip()
            chromium_area_counts[area] = chromium_area_counts.get(area, 0) + 1

            test_id = row.get("our_test_id", "").strip()
            if test_id:
                mapping_lookup[test_id] = row

            cat = row.get("failure_category", "").strip()
            if cat and cat != "not_ported":
                # Support multi-label categories (comma-separated)
                for part in cat.split(","):
                    part = part.strip()
                    if part:
                        failure_categories[part] = failure_categories.get(part, 0) + 1

    # Build per-area stats from summary tests
    area_stats = {}
    for t in summary["tests"]:
        area = extract_area(t["id"])
        if area not in area_stats:
            area_stats[area] = {
                "ported": 0,
                "passed": 0,
                "failed": 0,
                "mismatch_total": 0.0,
            }
        area_stats[area]["ported"] += 1
        if t["status"] == "pass":
            area_stats[area]["passed"] += 1
        else:
            area_stats[area]["failed"] += 1
        area_stats[area]["mismatch_total"] += t.get("mismatch_pct", 0.0)

    # Build the full test table rows
    test_rows = []
    seen_ids = set()

    for t in summary["tests"]:
        tid = t["id"]
        seen_ids.add(tid)
        area = extract_area(tid)
        row_data = {
            "id": tid,
            "area": area,
            "status": t["status"],
            "mismatch_pct": t.get("mismatch_pct", 0.0),
            "failure_category": "",
            "dependency": "",
        }
        if tid in mapping_lookup:
            m = mapping_lookup[tid]
            row_data["failure_category"] = m.get("failure_category", "")
            row_data["dependency"] = m.get("dependency", "")
        test_rows.append(row_data)

    # Add non-ported tests from mapping CSV (use chromium_test_path as unique key)
    if mapping:
        for row in mapping:
            test_id = row.get("our_test_id", "").strip()
            ported = row.get("ported", "").strip().lower()
            if ported != "true" and ported != "yes" and ported != "1":
                area = row.get("sp_area", "unknown").strip()
                chromium_path = row.get("chromium_test_path", "").strip()
                test_name = row.get("test_name", "").strip()
                # Use chromium_test_path as unique identifier to avoid flat-name collisions
                display_id = chromium_path if chromium_path else test_name
                if display_id and display_id not in seen_ids:
                    seen_ids.add(display_id)
                    test_rows.append({
                        "id": display_id,
                        "area": area,
                        "status": "not-ported",
                        "mismatch_pct": None,
                        "failure_category": row.get("failure_category", ""),
                        "dependency": row.get("dependency", ""),
                    })

    return {
        "chromium_total": chromium_total,
        "total_ported": total_ported,
        "total_passed": total_passed,
        "total_failed": total_failed,
        "area_stats": area_stats,
        "chromium_area_counts": chromium_area_counts,
        "failure_categories": failure_categories,
        "test_rows": test_rows,
        "has_mapping": mapping is not None,
        "has_sp12": sp12 is not None,
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
    }


def esc(s):
    return html.escape(str(s))


def generate_html(data):
    chromium_total = data["chromium_total"]
    total_ported = data["total_ported"]
    total_passed = data["total_passed"]
    total_failed = data["total_failed"]
    coverage_pct = (total_ported / chromium_total * 100) if chromium_total else 0
    pass_pct = (total_passed / total_ported * 100) if total_ported else 0

    # --- Summary Cards ---
    cards_html = f"""
    <div class="cards">
      <div class="card">
        <div class="card-label">Chromium WPT Tests</div>
        <div class="card-value">{chromium_total:,}</div>
        <div class="card-sub">total reference tests</div>
      </div>
      <div class="card">
        <div class="card-label">Ported</div>
        <div class="card-value accent-blue">{total_ported:,}</div>
        <div class="card-sub">{coverage_pct:.1f}% of Chromium</div>
      </div>
      <div class="card">
        <div class="card-label">Passing</div>
        <div class="card-value accent-green">{total_passed:,}</div>
        <div class="card-sub">{pass_pct:.1f}% of ported</div>
      </div>
      <div class="card">
        <div class="card-label">Failing</div>
        <div class="card-value accent-red">{total_failed:,}</div>
        <div class="card-sub">{100 - pass_pct:.1f}% of ported</div>
      </div>
      <div class="card">
        <div class="card-label">Coverage</div>
        <div class="card-value accent-blue">{coverage_pct:.1f}%</div>
        <div class="progress-bar"><div class="progress-fill bg-blue" style="width:{coverage_pct:.1f}%"></div></div>
      </div>
    </div>
    """

    # --- Per-Area Breakdown ---
    area_rows_html = ""
    for area in sorted(data["area_stats"].keys()):
        s = data["area_stats"][area]
        chromium_count = data["chromium_area_counts"].get(area, "—")
        pass_rate = (s["passed"] / s["ported"] * 100) if s["ported"] else 0
        avg_mismatch = s["mismatch_total"] / s["ported"] if s["ported"] else 0

        if pass_rate >= 80:
            bar_cls = "bg-green"
        elif pass_rate >= 40:
            bar_cls = "bg-yellow"
        else:
            bar_cls = "bg-red"

        area_rows_html += f"""
        <tr>
          <td class="mono">{esc(area)}</td>
          <td class="num">{chromium_count}</td>
          <td class="num">{s['ported']}</td>
          <td class="num">{s['passed']}</td>
          <td class="num">{s['failed']}</td>
          <td>
            <div class="rate-cell">
              <span class="rate-num">{pass_rate:.1f}%</span>
              <div class="progress-bar sm"><div class="progress-fill {bar_cls}" style="width:{pass_rate:.1f}%"></div></div>
            </div>
          </td>
          <td class="num">{avg_mismatch:.2f}%</td>
        </tr>"""

    area_table_html = f"""
    <div class="section">
      <h2>Per-Area Breakdown</h2>
      <table class="data-table" id="area-table">
        <thead>
          <tr>
            <th>Area</th>
            <th>Chromium Total</th>
            <th>Ported</th>
            <th>Passing</th>
            <th>Failing</th>
            <th>Pass Rate</th>
            <th>Avg Mismatch</th>
          </tr>
        </thead>
        <tbody>{area_rows_html}
        </tbody>
      </table>
    </div>
    """

    # --- Failure Category Breakdown ---
    failure_html = ""
    if data["failure_categories"]:
        cats = data["failure_categories"]
        total_cats = sum(cats.values())
        cat_rows = ""
        for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
            pct = count / total_cats * 100 if total_cats else 0
            cat_rows += f"""
            <div class="cat-row">
              <div class="cat-label">{esc(cat)}</div>
              <div class="cat-bar-wrap">
                <div class="cat-bar" style="width:{pct:.1f}%"></div>
              </div>
              <div class="cat-count">{count} ({pct:.1f}%)</div>
            </div>"""

        failure_html = f"""
        <div class="section">
          <h2>Failure Category Breakdown</h2>
          <div class="cat-chart">{cat_rows}
          </div>
        </div>
        """
    elif not data["has_mapping"]:
        failure_html = """
        <div class="section">
          <h2>Failure Category Breakdown</h2>
          <div class="notice">
            <p>⚠ Mapping CSV not yet available. Failure categories will appear once
            <code>wpt_mapping.csv</code> is generated.</p>
          </div>
        </div>
        """

    # --- Sortable Test Table ---
    test_rows_json = json.dumps(data["test_rows"])

    test_table_html = f"""
    <div class="section">
      <h2>All Tests</h2>
      <div class="table-controls">
        <input type="text" id="filter-input" placeholder="Filter by test ID, area, status, or category…"
               oninput="filterTable()">
        <div class="filter-badges">
          <button class="badge-btn all active" onclick="filterByStatus('all')">All ({len(data['test_rows'])})</button>
          <button class="badge-btn pass" onclick="filterByStatus('pass')">Pass ({total_passed})</button>
          <button class="badge-btn fail" onclick="filterByStatus('fail')">Fail ({total_failed})</button>
          <button class="badge-btn np" onclick="filterByStatus('not-ported')">Not Ported</button>
        </div>
      </div>
      <div class="table-scroll">
        <table class="data-table" id="test-table">
          <thead>
            <tr>
              <th onclick="sortTable(0)" class="sortable">Test ID ⇅</th>
              <th onclick="sortTable(1)" class="sortable">Area ⇅</th>
              <th onclick="sortTable(2)" class="sortable">Status ⇅</th>
              <th onclick="sortTable(3)" class="sortable">Mismatch % ⇅</th>
              <th onclick="sortTable(4)" class="sortable">Failure Category ⇅</th>
              <th onclick="sortTable(5)" class="sortable">Dependency ⇅</th>
            </tr>
          </thead>
          <tbody id="test-tbody"></tbody>
        </table>
      </div>
      <div class="table-info" id="table-info"></div>
    </div>
    """

    # --- Full HTML ---
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Open UI — WPT Test Dashboard</title>
<style>
*, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, sans-serif;
  background: #f0f2f5;
  color: #1a1a2e;
  line-height: 1.5;
}}
.header {{
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  color: #fff;
  padding: 2rem 1.5rem 1.5rem;
  text-align: center;
}}
.header h1 {{ font-size: 1.8rem; font-weight: 700; letter-spacing: -0.02em; }}
.header p {{ opacity: 0.75; margin-top: 0.3rem; font-size: 0.95rem; }}
.container {{ max-width: 1280px; margin: 0 auto; padding: 1.5rem; }}

/* Summary Cards */
.cards {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}}
.card {{
  background: #fff;
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  box-shadow: 0 1px 3px rgba(0,0,0,0.08), 0 4px 12px rgba(0,0,0,0.04);
  text-align: center;
}}
.card-label {{ font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.06em; color: #6c757d; font-weight: 600; }}
.card-value {{ font-size: 2.2rem; font-weight: 800; margin: 0.25rem 0; }}
.card-sub {{ font-size: 0.82rem; color: #6c757d; }}
.accent-green {{ color: #28a745; }}
.accent-red {{ color: #dc3545; }}
.accent-blue {{ color: #0d6efd; }}
.accent-yellow {{ color: #ffc107; }}

/* Progress bars */
.progress-bar {{
  background: #e9ecef;
  border-radius: 999px;
  height: 8px;
  overflow: hidden;
  margin-top: 0.5rem;
}}
.progress-bar.sm {{ height: 6px; margin-top: 0; }}
.progress-fill {{ height: 100%; border-radius: 999px; transition: width 0.4s ease; }}
.bg-green {{ background: #28a745; }}
.bg-red {{ background: #dc3545; }}
.bg-yellow {{ background: #ffc107; }}
.bg-blue {{ background: #0d6efd; }}

/* Sections */
.section {{
  background: #fff;
  border-radius: 12px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  box-shadow: 0 1px 3px rgba(0,0,0,0.08), 0 4px 12px rgba(0,0,0,0.04);
}}
.section h2 {{ font-size: 1.2rem; font-weight: 700; margin-bottom: 1rem; color: #1a1a2e; }}

/* Data tables */
.data-table {{
  width: 100%;
  border-collapse: collapse;
  font-size: 0.88rem;
}}
.data-table th {{
  text-align: left;
  padding: 0.6rem 0.75rem;
  background: #f8f9fa;
  border-bottom: 2px solid #dee2e6;
  font-weight: 600;
  color: #495057;
  white-space: nowrap;
  position: sticky;
  top: 0;
  z-index: 1;
}}
.data-table th.sortable {{ cursor: pointer; user-select: none; }}
.data-table th.sortable:hover {{ background: #e9ecef; }}
.data-table td {{
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid #f0f0f0;
  vertical-align: middle;
}}
.data-table tbody tr:hover {{ background: #f8f9fb; }}
.num {{ text-align: right; font-variant-numeric: tabular-nums; }}
.mono {{ font-family: 'SF Mono', SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.82rem; word-break: break-all; }}
.rate-cell {{ display: flex; align-items: center; gap: 0.5rem; }}
.rate-num {{ font-weight: 600; min-width: 3.5em; text-align: right; font-variant-numeric: tabular-nums; }}
.rate-cell .progress-bar {{ flex: 1; min-width: 60px; }}

/* Status badges */
.badge {{
  display: inline-block;
  padding: 0.15rem 0.55rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}}
.badge-pass {{ background: #d4edda; color: #155724; }}
.badge-fail {{ background: #f8d7da; color: #721c24; }}
.badge-np {{ background: #e2e3e5; color: #383d41; }}

/* Category chart */
.cat-chart {{ max-width: 700px; }}
.cat-row {{ display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.6rem; }}
.cat-label {{ min-width: 160px; font-size: 0.85rem; font-weight: 500; text-align: right; font-family: 'SF Mono', SFMono-Regular, Consolas, monospace; }}
.cat-bar-wrap {{ flex: 1; background: #e9ecef; border-radius: 4px; height: 20px; overflow: hidden; }}
.cat-bar {{ height: 100%; background: linear-gradient(90deg, #dc3545 0%, #e4606d 100%); border-radius: 4px; transition: width 0.4s ease; }}
.cat-count {{ min-width: 90px; font-size: 0.82rem; color: #6c757d; font-variant-numeric: tabular-nums; }}

/* Notice */
.notice {{
  background: #fff3cd;
  border: 1px solid #ffc107;
  border-radius: 8px;
  padding: 1rem 1.25rem;
  font-size: 0.9rem;
  color: #856404;
}}
.notice code {{ background: #ffeeba; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.85rem; }}

/* Table controls */
.table-controls {{
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-bottom: 1rem;
  align-items: center;
}}
#filter-input {{
  flex: 1;
  min-width: 250px;
  padding: 0.55rem 0.85rem;
  border: 1px solid #ced4da;
  border-radius: 8px;
  font-size: 0.9rem;
  outline: none;
  transition: border-color 0.15s;
}}
#filter-input:focus {{ border-color: #0d6efd; box-shadow: 0 0 0 3px rgba(13,110,253,0.15); }}
.filter-badges {{ display: flex; gap: 0.4rem; }}
.badge-btn {{
  padding: 0.35rem 0.7rem;
  border: 1px solid #dee2e6;
  border-radius: 999px;
  background: #fff;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}}
.badge-btn:hover {{ background: #f0f0f0; }}
.badge-btn.active {{ background: #1a1a2e; color: #fff; border-color: #1a1a2e; }}
.badge-btn.pass.active {{ background: #28a745; border-color: #28a745; }}
.badge-btn.fail.active {{ background: #dc3545; border-color: #dc3545; }}
.badge-btn.np.active {{ background: #6c757d; border-color: #6c757d; }}
.table-scroll {{ max-height: 600px; overflow-y: auto; border: 1px solid #e9ecef; border-radius: 8px; }}
.table-info {{ margin-top: 0.5rem; font-size: 0.82rem; color: #6c757d; }}

/* Footer */
.footer {{
  text-align: center;
  padding: 2rem 1rem;
  font-size: 0.8rem;
  color: #6c757d;
}}

/* Responsive */
@media (max-width: 768px) {{
  .cards {{ grid-template-columns: repeat(2, 1fr); }}
  .header h1 {{ font-size: 1.3rem; }}
  .card-value {{ font-size: 1.6rem; }}
  .table-controls {{ flex-direction: column; }}
  .cat-label {{ min-width: 100px; font-size: 0.75rem; }}
}}
</style>
</head>
<body>
<div class="header">
  <h1>Open UI — WPT Test Dashboard</h1>
  <p>Web Platform Test coverage and pixel-comparison results</p>
</div>
<div class="container">
{cards_html}
{area_table_html}
{failure_html}
{test_table_html}
</div>
<div class="footer">
  Generated {esc(data['timestamp'])} &middot; Open UI WPT Accountability Tracker
  {' &middot; <em>Mapping CSV not loaded — showing summary.json data only</em>' if not data['has_mapping'] else ''}
</div>

<script>
// Test data
var allTests = {test_rows_json};
var currentStatusFilter = 'all';
var currentSort = {{ col: -1, asc: true }};

function badgeHTML(status) {{
  if (status === 'pass') return '<span class="badge badge-pass">PASS</span>';
  if (status === 'fail') return '<span class="badge badge-fail">FAIL</span>';
  return '<span class="badge badge-np">NOT PORTED</span>';
}}

function renderTable(rows) {{
  var tbody = document.getElementById('test-tbody');
  var html = '';
  var limit = 8000;
  var shown = Math.min(rows.length, limit);
  for (var i = 0; i < shown; i++) {{
    var r = rows[i];
    var mm = r.mismatch_pct !== null && r.mismatch_pct !== undefined ? r.mismatch_pct.toFixed(2) + '%' : '—';
    html += '<tr>'
      + '<td class="mono">' + escHTML(r.id) + '</td>'
      + '<td>' + escHTML(r.area) + '</td>'
      + '<td>' + badgeHTML(r.status) + '</td>'
      + '<td class="num">' + mm + '</td>'
      + '<td>' + escHTML(r.failure_category || '') + '</td>'
      + '<td>' + escHTML(r.dependency || '') + '</td>'
      + '</tr>';
  }}
  tbody.innerHTML = html;
  document.getElementById('table-info').textContent =
    'Showing ' + shown + ' of ' + rows.length + ' tests' + (rows.length > limit ? ' (limited to ' + limit + ')' : '');
}}

function escHTML(s) {{
  var d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}}

function getFiltered() {{
  var text = document.getElementById('filter-input').value.toLowerCase();
  return allTests.filter(function(r) {{
    if (currentStatusFilter !== 'all' && r.status !== currentStatusFilter) return false;
    if (!text) return true;
    return (r.id + ' ' + r.area + ' ' + r.status + ' ' + (r.failure_category || '') + ' ' + (r.dependency || '')).toLowerCase().indexOf(text) !== -1;
  }});
}}

function filterTable() {{
  var rows = getFiltered();
  if (currentSort.col >= 0) rows = doSort(rows, currentSort.col, currentSort.asc);
  renderTable(rows);
}}

function filterByStatus(status) {{
  currentStatusFilter = status;
  document.querySelectorAll('.badge-btn').forEach(function(b) {{ b.classList.remove('active'); }});
  document.querySelector('.badge-btn.' + (status === 'not-ported' ? 'np' : status === 'all' ? 'all' : status)).classList.add('active');
  filterTable();
}}

function sortKey(r, col) {{
  switch(col) {{
    case 0: return r.id;
    case 1: return r.area;
    case 2: return r.status;
    case 3: return r.mismatch_pct !== null && r.mismatch_pct !== undefined ? r.mismatch_pct : -1;
    case 4: return r.failure_category || '';
    case 5: return r.dependency || '';
  }}
  return '';
}}

function doSort(rows, col, asc) {{
  return rows.slice().sort(function(a, b) {{
    var va = sortKey(a, col), vb = sortKey(b, col);
    if (typeof va === 'number' && typeof vb === 'number') {{
      return asc ? va - vb : vb - va;
    }}
    va = String(va).toLowerCase();
    vb = String(vb).toLowerCase();
    if (va < vb) return asc ? -1 : 1;
    if (va > vb) return asc ? 1 : -1;
    return 0;
  }});
}}

function sortTable(col) {{
  if (currentSort.col === col) {{
    currentSort.asc = !currentSort.asc;
  }} else {{
    currentSort.col = col;
    currentSort.asc = true;
  }}
  filterTable();
}}

// Initial render
filterTable();
</script>
</body>
</html>"""


def main():
    data = build_report_data()
    html_content = generate_html(data)
    os.makedirs(os.path.dirname(OUTPUT_HTML), exist_ok=True)
    with open(OUTPUT_HTML, "w") as f:
        f.write(html_content)
    print(f"Report generated: {OUTPUT_HTML}")
    print(f"  Chromium total: {data['chromium_total']:,}")
    print(f"  Ported: {data['total_ported']:,}")
    print(f"  Passing: {data['total_passed']:,}")
    print(f"  Failing: {data['total_failed']:,}")
    print(f"  Mapping CSV: {'loaded' if data['has_mapping'] else 'not available'}")
    print(f"  SP12.5 CSV: {'loaded' if data['has_sp12'] else 'not available'}")
    print(f"  Test rows in table: {len(data['test_rows']):,}")


if __name__ == "__main__":
    main()
