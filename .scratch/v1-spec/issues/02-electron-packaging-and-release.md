# Electron packaging and cross-platform release

Type: research
Status: claimed
Blocked by: —
Findings: ../research/02-electron-packaging-and-release.md

## Question

What is the current state of the art for packaging and releasing an **unsigned** Electron
application to Linux, Windows and macOS from GitHub Actions?

- electron-builder versus Electron Forge — which is the live default, and why
- which artifact formats to ship per OS (AppImage / deb / rpm; NSIS / portable exe; dmg / zip)
- exactly what the end user sees when launching an unsigned build on each OS, and the
  documented steps to get past it
- what a GitHub Actions release workflow looks like for a build matrix across three runners
- which repository settings and token permissions a first-time Actions user must configure by
  hand, and which are defaults

Development happens on Fedora Linux, so macOS and Windows artifacts must come from CI runners.
