# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project doesn't care about versioning.

## [Unreleased]

### Added
- Add support for official VSCode packages (see [GH-18]).
- Add support for systemd log control interface, in order to change log level and log target at runtime with `systemctl service-log-level` and `systemctl service-log-target` respectively (see [GH-19]).

## Changed
- Use tracing for logging (see [GH-19]).
- Change systemd service name to `gnome-search-providers-vscode.service` (see [GH-20]).

[GH-18]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/18
[GH-19]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/19
[GH-20]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/20

## [1.6.0] – 2021-11-27

### Added
- Add support for multi-root workspaces, a.k.a. `.code-workspace` files (see [GH-15]).

### Changed
- Use async IO.

### Removed
- Dedicated AUR package support for VSCodium; the AUR package now follows standard names.

[GH-15]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/15

## [1.5.0] – 2021-09-25

### Added
- Add support for general Linux codium (see [GH-13]).

[GH-13]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/13

## [1.4.0] – 2021-09-08

### Added
- Add support for VSCodium (see [GH-12]).

### Changed
- The systemd service now logs directly to the systemd journal; this improves representation of log levels in logging.

[GH-12]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/12

## [1.3.0] – 2021-05-16

### Changed
- Use common code from [gnome-search-providers-jetbrains](https://github.com/lunaryorn/gnome-search-providers-jetbrains/tree/main/crates/common):
  - The search provider now moves launched processes to new `app-gnome` systemd scopes, like Gnome itself does when starting applications
  - The search provider now runs in a glib mainloop.

### Fixed
- No longer quit application instances launched by the search provider when stopping the search provider service; the search provider now moves processes to new systemd scopes to prevent this.

## [1.2.0] – 2021-04-26

### Changed

- Improve order of matches (see [GH-2]):
    - Rank matches in the workspace name higher than matches in the path, and
    - rank URL matches by position of term in match (the more to the right the better the term matched the more specific segments of the URL).

[GH-2]: https://github.com/lunaryorn/gnome-search-providers-vscode/issues/2

## [1.1.1] – 2021-04-23

### Fixed
- Make sure to build before `make install` (see [GH-8]).

[GH-8]: https://github.com/lunaryorn/gnome-search-providers-vscode/issues/8

## [1.1.0] – 2021-04-22

### Added

- Support AUR VSCode binary (see [GH-6], thanks [SantoJambit]).
- Support for storage format of VSCode 1.55 (see [GH-5], thanks [SantoJambit])

### Fixed

- Exit with failure if the bus name is already owned by another process.
- Substitude prefix in service files (see [GH-4], thanks [SantoJambit]).
- Maintain order of workspaces in results (see [GH-7]).

[SantoJambit]: https://github.com/SantoJambit
[GH-4]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/4
[GH-5]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/5
[GH-6]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/6
[GH-7]: https://github.com/lunaryorn/gnome-search-providers-vscode/pull/7

## [1.0.0] – 2021-04-18

Initial release with support for workspaces of Code - OSS from Arch Linux.

[Unreleased]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.6.0...HEAD
[1.6.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/releases/tag/v1.0.0
