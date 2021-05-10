# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project doesn't care about versioning.

## [Unreleased]

### Changed
- Run with the glib mainloop (see [GH-9]).

[GH-9]: https://github.com/lunaryorn/gnome-search-providers-vscode/issues/9

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

[Unreleased]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/lunaryorn/gnome-search-providers-vscode/releases/tag/v1.0.0
