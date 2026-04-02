# Sub-Project 9: Platform Expansion & Ecosystem

> Extend Open UI to macOS, Windows, and mobile platforms. Build ecosystem tooling.

## Objective

Open UI launches on Linux (X11/Wayland). This sub-project extends to macOS (Metal backend), Windows (D3D12 backend), and mobile (Android/iOS). Also builds ecosystem tooling: package manager integration, CI templates, visual inspector, and community resources.

## Tasks

### Phase A: macOS
1. macOS windowing (NSWindow, CAMetalLayer)
2. Metal rendering backend for Skia/cc/
3. macOS input handling, HiDPI, dark mode
4. macOS packaging (.app bundle)

### Phase B: Windows
5. Windows windowing (HWND, DXGI)
6. D3D12/D3D11 rendering backend
7. Windows input, HiDPI, theme integration
8. Windows packaging (.exe, MSIX)

### Phase C: Mobile (Stretch)
9. Android (Surface, Vulkan)
10. iOS (UIKit, Metal)

### Phase D: Ecosystem
11. Package manager integration (system packages, vcpkg, conan)
12. CI/CD templates (GitHub Actions, GitLab CI)
13. Visual inspector tool (element tree, style, layout visualization)
14. Performance profiling integration (Chrome tracing format)
15. Community: website, docs site, Discord/forums

## Deliverables

| Deliverable | Description |
|---|---|
| macOS support | Full rendering pipeline on macOS |
| Windows support | Full rendering pipeline on Windows |
| Platform CI | Cross-platform CI/CD |
| Inspector tool | Visual debugging tool |
| Docs site | Project website with guides |

## Success Criteria

- [ ] Same application source renders identically on Linux, macOS, Windows
- [ ] Native platform feel (HiDPI, dark mode, input conventions)
- [ ] CI builds and tests on all platforms
- [ ] Inspector tool shows live element tree and computed styles
