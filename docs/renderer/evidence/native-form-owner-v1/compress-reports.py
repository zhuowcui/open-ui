"""Store large new reports with deterministic, lossless gzip compression."""
import gzip
import hashlib
import io
import json
from pathlib import Path

DIRECTORY = Path(__file__).resolve().parent
SUMMARY = DIRECTORY.parent.parent / 'generated/native-form-owner-v1.json'
report = json.loads(SUMMARY.read_bytes())
for name in ['native-receipt.json', 'full-summary.json', 'expanded-summary.json']:
    path = DIRECTORY / name
    if not path.exists():
        continue
    data = path.read_bytes()
    original = report['files'].pop(name)
    assert hashlib.sha256(data).hexdigest() == original['sha256']
    buffer = io.BytesIO()
    with gzip.GzipFile(filename='', mode='wb', fileobj=buffer, mtime=0) as stream:
        stream.write(data)
    compressed = buffer.getvalue()
    assert gzip.decompress(compressed) == data
    target = path.with_name(name + '.gz')
    assert not target.exists()
    target.write_bytes(compressed)
    report['files'][target.name] = dict(
        origin=original['origin'], sha256=hashlib.sha256(compressed).hexdigest(),
        bytes=len(compressed), compression='gzip',
        original_sha256=original['sha256'], original_bytes=len(data),
        original_report_bytes_preserved=True)
    path.unlink()
script = Path(__file__)
report['files'][script.name] = dict(origin='repository evidence packaging',
    sha256=hashlib.sha256(script.read_bytes()).hexdigest(), bytes=script.stat().st_size)
SUMMARY.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
