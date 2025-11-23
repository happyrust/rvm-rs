# Changelog

All notable changes to the RVM Rust port will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Complete RVM file parser with support for all geometry types
- ATT attribute file parser
- Store system for managing scene graph hierarchy
- Export functionality for OBJ, glTF/GLB, and JSON formats
- Tessellation system for all primitive geometry types:
  - Pyramid, Box, Cylinder, Sphere
  - Circular Torus, Rectangular Torus
  - Elliptical Dish, Spherical Dish
  - Snout, Line, FacetGroup
- Scale-aware tessellation matching C++ implementation
- Transparency support for OBST and INSU geometry types
- Material and color management
- Visitor pattern for scene graph traversal

### Fixed
- Matrix construction from RVM file format (column-major order)
- Scale extraction from transformation matrices
- Sagitta-based segment count calculation for adaptive tessellation
- Transparency inheritance from parent groups
- FacetGroup parsing with proper contour handling
- All 11 geometry types now parse correctly with proper fallback

### Changed
- Tessellate trait now accepts scale parameter for accurate subdivision
- All geometry tessellation functions use scale-aware segment calculation
- Improved error handling with specific ParseError types

## [0.1.0] - 2024-11-23

### Added
- Initial project structure
- Basic RVM parser skeleton
- Store and geometry data structures
- Export module foundation
