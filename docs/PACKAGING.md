# Packaging options

As of 2026-10-19. Minimum versions according to `meson.build`: GTK ≥ 4.18, libadwaita ≥ 1.8
(AdwShortcutsDialog), VTE (gtk4) ≥ 0.80 (termprops, context menu model). Labels:
**F** fact (built/tested myself) · **S** inference · **V** proposal.

## Overview

| Option | Status | Effort | Assessment |
|---|---|---|---|
| **Arch, own repo `linuxundich`** | **F** working: `services/pacman-repo/pkgbuilds/parkett`, Meson + `check()`, published on bender (r14) | done | Main route for the author's own machines |
| Arch, AUR | – | low | **S** the public repository now exists; PKGBUILD is almost ready |
| **Fedora** (RPM/COPR) | **F** builds and runs on Fedora 44 (GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1) from the source tree with Meson | medium | **V** spec file + COPR once there is a public source |
| Debian/Ubuntu (.deb) | untested | medium | **S** Debian 13 and Ubuntu 24.04 have libadwaita/VTE versions that are too old; newer ones (Ubuntu 26.04, Debian forky, TUXEDO OS) probably fit – check in the existing VMs (`ubuntu26.04`, `tuxedo-os-forky`). Lowering the requirement to libadwaita 1.7 would be possible if the shortcuts overview did without AdwShortcutsDialog |
| **Flatpak** | – | **high** | see below |
| AppImage, Snap | – | – | **V** do not pursue: a terminal needs host access and system integration |

**Nautilus extension:** `share/nautilus-python/extensions/parkett-nautilus.py`, only works with
`nautilus-python` (Arch: optional dependency in the PKGBUILD). Uses the path to `parkett` from
Meson (`@BINDIR@`); in a Flatpak it would have to call `flatpak run` and would live outside
the sandbox.

## Flatpak in detail

**F** Parkett currently uses the host directly: it starts the shell via `vte_terminal_spawn_async`
and reads `/proc` (foreground process for the confirmation on close, root/SSH context color,
"Save as template"). Inside the Flatpak sandbox it would only see its own PID namespace.

**S** What would be needed:
- Start the shell on the host: `flatpak-spawn --host` or a host PTY via
  `org.freedesktop.Flatpak.Development` (permission `--talk-name=org.freedesktop.Flatpak`).
- A **host agent** as in Ptyxis (`agent/`, D-Bus over `socketpair`, methods including
  `CreatePty`, `Spawn`, `HasForegroundProcess`, `GetWorkingDirectory`), linked against an old glibc
  so it runs on any host. Without it, the confirmation on close, the
  context color and template commands are lost.
- VTE is not part of the GNOME runtime and would have to be built as a module.

**Flathub:** By the author's own rule, AI-assisted apps are not submitted there; a separate
Flatpak repo (as with other projects in `apps/*`) would remain possible.

**V** Only tackle Flatpak when there is a reason (other distributions without suitable
packages); then as a separate step with a host agent.

## Recommendation

**Status 2026-10-23:** version 0.4.0 is public on GitHub (source only). Binary packages exist
only in the author's private Arch repo; for everyone else the route is building from source
([BUILDING.md](BUILDING.md)).

1. Next: AUR and Fedora COPR from the public repository.
2. The private Arch repo stays for the author's own machines.
3. .deb only if needed, after testing in the Ubuntu/TUXEDO VMs.
4. Flatpak only with a host agent, as a separate project.
