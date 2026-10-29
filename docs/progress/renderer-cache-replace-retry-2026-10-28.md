# Renderer cache replacement on a Windows-backed checkout, 28 October 2026

The renderer matrix now retries a cache entry replacement that the
filesystem refuses while the file is briefly held open. Census runs in WSL
against a checkout on the Windows drive no longer abort on it. Renderer
output and the cache format are unchanged.

## Finding

The paired census for this round runs `run_renderer_matrix.py` in WSL
against `/mnt/e/open-ui`, a Windows NTFS drive mounted through DrvFs. Four
of the last five first runs stopped in `store_cached_result`:

```text
PermissionError: [Errno 13] Permission denied:
'/mnt/e/open-ui/out/renderer-qualification-cache/d3/.d3a4…69b.128518239286976.tmp'
-> '/mnt/e/open-ui/out/renderer-qualification-cache/d3/d3a4…69b.json'
```

`store_cached_result` writes the entry to a temporary file and moves it into
place with `Path.replace`. On DrvFs that rename fails with `EACCES` while
another process holds either file open. Candidates are a concurrent reader,
or Windows on-access scanning of the newly written file. Rerunning only the
matrix succeeded each time, so the hold is transient. The other cache
writers in the file, `store_chromium_oracle` and
`store_content_addressed_png`, publish with `os.link` and did not fail.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `store_cached_result` retries `Path.replace` on `PermissionError` up to `CACHE_REPLACE_ATTEMPTS` (40) times, with a growing delay capped at 0.5 s. After the last attempt it removes the temporary file and raises the error. | renderer qualification tooling | `tools/qualification/run_renderer_matrix.py` |

Two tests in `tools/qualification/test_renderer_matrix.py` cover it:

- `test_transient_permission_error_is_retried` fails the replacement twice,
  then stores the entry and leaves no temporary file.
- `test_persistent_permission_error_fails_without_leaving_temporary`
  raises after every attempt is refused. No entry and no temporary file
  remain.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Matrix tests on Windows | `python -m unittest tools.qualification.test_renderer_matrix` | 37 run, OK, 1 skipped |
| Matrix tests in WSL | `python3 -m unittest tools.qualification.test_renderer_matrix` | 37 run, OK, 1 skipped |
