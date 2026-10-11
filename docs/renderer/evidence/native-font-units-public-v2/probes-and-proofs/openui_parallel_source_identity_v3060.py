"""Read every source byte; preserve qualification hash framing/order."""
import hashlib, os, subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root)

def identity(root, workers=12):
    root=Path(root)
    before=git(root,"status","--porcelain=v1","-z","--untracked-files=all")
    commit=git(root,"rev-parse","HEAD").decode().strip()
    paths=sorted(p for p in git(root,"ls-files","-co","--exclude-standard","-z").split(b"\0") if p)
    def read(encoded):
        path=root/os.fsdecode(encoded)
        if not path.exists() and not path.is_symlink(): content=b"<deleted>"
        elif path.is_symlink():content=os.readlink(path).encode("utf-8","surrogateescape")
        else: content=path.read_bytes()
        return encoded,content
    digest=hashlib.sha256()
    with ThreadPoolExecutor(max_workers=workers) as pool:
        for name,content in pool.map(read,paths):
            digest.update(len(name).to_bytes(8,"big"));digest.update(name)
            digest.update(len(content).to_bytes(8,"big"));digest.update(content)
    harness=hashlib.sha256()
    harness_paths=[*sorted((root/"tools/qualification").glob("*.py")),root/"tools/accountability/run_all_pixel_comparisons.py",root/"bindings/rust/pixel-compare/Cargo.toml",root/"bindings/rust/pixel-compare/src/main.rs",root/"bindings/rust/pixel-compare/src/wpt/mod.rs"]
    for path in harness_paths:
        name=str(path.relative_to(root)).encode();content=path.read_bytes()
        harness.update(len(name).to_bytes(8,"big"));harness.update(name);harness.update(len(content).to_bytes(8,"big"));harness.update(content)
    assert git(root,"status","--porcelain=v1","-z","--untracked-files=all")==before
    assert git(root,"rev-parse","HEAD").decode().strip()==commit
    return dict(commit=commit,clean=not before,status_sha256=hashlib.sha256(before).hexdigest(),source_tree_sha256=digest.hexdigest(),harness_sha256=harness.hexdigest())
