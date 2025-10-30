# Change Log

All notable changes to a5-rs will be documented in this file.

For the latest documentation, visit [A5 Documentation](https://a5geo.org)

<!--
Each version should:
  List its release date in the above format.
  Group changes to describe their impact on the project, as follows:
  Added for new features.
  Changed for changes in existing functionality.
  Deprecated for once-stable features removed in upcoming releases.
  Removed for deprecated features removed in this release.
  Fixed for any bug fixes.
  Security to invite users to upgrade in case of vulnerabilities.
Ref: http://keepachangelog.com/en/0.3.0/
-->

## a5-rs

#### a5-rs [v0.6.0] - Oct 30 2025

- Feature: cell compaction/uncompaction (#35)

#### a5-rs [v0.5.1] - Oct 12 2025

- fix: support older Rust compiler versions back to 1.86.0 (#32)

#### a5-rs [v0.5.0] - Sep 21 2025

- Changed: Version bump to align with TypeScript and Python implementations

## a5-rs v0.4

#### a5-rs [v0.4.3] - Aug 31 2025

- Fixed: Avoid warnings when publishing (#29)
- Removed: bigint dependency (#28)
- Added: README documentation (#27)
- Changed: Remove 0x check (#26)
- Added: Public API finalization (#25)
- Changed: Prepare for publishing (#24)

#### a5-rs [v0.4.2] - Aug 28 2025

- Added: 10X speed improvement by global projection instance (#23)
- Removed: TilingShape and TriangleShape (#22)
- Added: Port cell functions (#21)

#### a5-rs [v0.4.1] - Aug 22 2025

- Added: Port dodecahedron projection (#20)
- Added: Port CRS (#19)
- Added: Port polyhedral projection (#18)
- Added: Coordinate transforms (#17)
- Added: Port tiling (#16)
- Added: Port pentagon & serialization (#15)
- Added: Port origin & dodecahedron quat (#14)

#### a5-rs [v0.4.0] - Aug 20 2025

- Added: Port core/utils (#13)
- Added: Spherical triangle (#12)
- Added: Spherical polygon port (#11)
- Added: Port hilbert (#10)
- Added: Port PentagonShape class (#9)
- Added: Port vector code (#8)
- Added: Authalic projection (#7)
- Added: Port constants (#6)
- Added: Port the gnomonic projection (#5)
- Added: Coordinate systems and gnomonic transformations (#4)

## a5-rs v0.3

#### a5-rs [v0.3.0] - Aug 7 2025

- Changed: Update GitHub workflows (#3)

## a5-rs v0.2

#### a5-rs [v0.2.0] - May 25 2025

- Added: Setup CI (#2)

## a5-rs v0.1

#### a5-rs [v0.1.0] - May 25 2025

- Added: Initial Rust implementation of A5 - Global Pentagonal Geospatial Index
- Added: Implementation of hex functions (#1)
- Added: Project initialization
