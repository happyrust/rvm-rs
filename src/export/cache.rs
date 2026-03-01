use crate::export::tessellator::Triangulation;
use crate::store::geometry::GeometryKind;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A key for caching tessellated geometry
/// Only includes the geometry parameters that affect tessellation
#[derive(Debug, Clone)]
struct GeometryCacheKey {
    kind_hash: u64,
}

impl GeometryCacheKey {
    fn from_geometry(kind: &GeometryKind, tolerance: f32, scale: f32) -> Self {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the geometry kind and parameters
        match kind {
            GeometryKind::Pyramid(p) => {
                "Pyramid".hash(&mut hasher);
                p.bottom[0].to_bits().hash(&mut hasher);
                p.bottom[1].to_bits().hash(&mut hasher);
                p.top[0].to_bits().hash(&mut hasher);
                p.top[1].to_bits().hash(&mut hasher);
                p.offset[0].to_bits().hash(&mut hasher);
                p.offset[1].to_bits().hash(&mut hasher);
                p.height.to_bits().hash(&mut hasher);
            }
            GeometryKind::Box(b) => {
                "Box".hash(&mut hasher);
                b.lengths[0].to_bits().hash(&mut hasher);
                b.lengths[1].to_bits().hash(&mut hasher);
                b.lengths[2].to_bits().hash(&mut hasher);
            }
            GeometryKind::Cylinder(c) => {
                "Cylinder".hash(&mut hasher);
                c.radius.to_bits().hash(&mut hasher);
                c.height.to_bits().hash(&mut hasher);
            }
            GeometryKind::Sphere(s) => {
                "Sphere".hash(&mut hasher);
                s.radius.to_bits().hash(&mut hasher);
            }
            GeometryKind::RectangularTorus(t) => {
                "RectangularTorus".hash(&mut hasher);
                t.inner_radius.to_bits().hash(&mut hasher);
                t.outer_radius.to_bits().hash(&mut hasher);
                t.height.to_bits().hash(&mut hasher);
                t.angle.to_bits().hash(&mut hasher);
            }
            GeometryKind::CircularTorus(t) => {
                "CircularTorus".hash(&mut hasher);
                t.offset.to_bits().hash(&mut hasher);
                t.radius.to_bits().hash(&mut hasher);
                t.angle.to_bits().hash(&mut hasher);
            }
            GeometryKind::Snout(s) => {
                "Snout".hash(&mut hasher);
                s.radius_bottom.to_bits().hash(&mut hasher);
                s.radius_top.to_bits().hash(&mut hasher);
                s.height.to_bits().hash(&mut hasher);
                s.offset_x.to_bits().hash(&mut hasher);
                s.offset_y.to_bits().hash(&mut hasher);
                s.bottom_shear_x.to_bits().hash(&mut hasher);
                s.bottom_shear_y.to_bits().hash(&mut hasher);
                s.top_shear_x.to_bits().hash(&mut hasher);
                s.top_shear_y.to_bits().hash(&mut hasher);
            }
            GeometryKind::EllipticalDish(d) => {
                "EllipticalDish".hash(&mut hasher);
                d.base_radius.to_bits().hash(&mut hasher);
                d.height.to_bits().hash(&mut hasher);
            }
            GeometryKind::SphericalDish(d) => {
                "SphericalDish".hash(&mut hasher);
                d.base_radius.to_bits().hash(&mut hasher);
                d.height.to_bits().hash(&mut hasher);
            }
            GeometryKind::Line(l) => {
                "Line".hash(&mut hasher);
                l.start_radius.to_bits().hash(&mut hasher);
                l.end_radius.to_bits().hash(&mut hasher);
            }
            GeometryKind::FacetGroup(_) => {
                // FacetGroups are not cached (too complex and usually unique)
                "FacetGroup_NOCACHE".hash(&mut hasher);
                // Add a random component to ensure no caching
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .hash(&mut hasher);
            }
        }

        // Include tolerance and scale in the hash
        tolerance.to_bits().hash(&mut hasher);
        scale.to_bits().hash(&mut hasher);

        Self {
            kind_hash: hasher.finish(),
        }
    }
}

impl PartialEq for GeometryCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.kind_hash == other.kind_hash
    }
}

impl Eq for GeometryCacheKey {}

impl Hash for GeometryCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind_hash.hash(state);
    }
}

/// Cache for tessellated geometry
pub struct GeometryCache {
    cache: HashMap<GeometryCacheKey, Triangulation>,
    hits: usize,
    misses: usize,
}

impl GeometryCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            hits: 0,
            misses: 0,
        }
    }

    /// Get a cached triangulation or None if not found
    pub fn get(
        &mut self,
        kind: &GeometryKind,
        tolerance: f32,
        scale: f32,
    ) -> Option<Triangulation> {
        let key = GeometryCacheKey::from_geometry(kind, tolerance, scale);

        if let Some(tri) = self.cache.get(&key) {
            self.hits += 1;
            Some(tri.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a triangulation into the cache
    pub fn insert(&mut self, kind: &GeometryKind, tolerance: f32, scale: f32, tri: Triangulation) {
        let key = GeometryCacheKey::from_geometry(kind, tolerance, scale);
        self.cache.insert(key, tri);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Get the number of cached items
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for GeometryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub size: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::geometry::{Cylinder, Sphere};

    #[test]
    fn test_cache_basic() {
        let mut cache = GeometryCache::new();

        let cylinder = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });

        // First access should be a miss
        assert!(cache.get(&cylinder, 0.01, 1.0).is_none());

        // Insert a triangulation
        let tri = Triangulation::new();
        cache.insert(&cylinder, 0.01, 1.0, tri);

        // Second access should be a hit
        assert!(cache.get(&cylinder, 0.01, 1.0).is_some());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_cache_different_parameters() {
        let mut cache = GeometryCache::new();

        let cylinder1 = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });

        let cylinder2 = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 3.0, // Different height
        });

        cache.insert(&cylinder1, 0.01, 1.0, Triangulation::new());

        // Same geometry, same parameters - should hit
        assert!(cache.get(&cylinder1, 0.01, 1.0).is_some());

        // Different geometry - should miss
        assert!(cache.get(&cylinder2, 0.01, 1.0).is_none());

        // Same geometry, different tolerance - should miss
        assert!(cache.get(&cylinder1, 0.001, 1.0).is_none());

        // Same geometry, different scale - should miss
        assert!(cache.get(&cylinder1, 0.01, 2.0).is_none());
    }

    #[test]
    fn test_cache_different_geometry_types() {
        let mut cache = GeometryCache::new();

        let cylinder = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });

        let sphere = GeometryKind::Sphere(Sphere { radius: 1.0 });

        cache.insert(&cylinder, 0.01, 1.0, Triangulation::new());
        cache.insert(&sphere, 0.01, 1.0, Triangulation::new());

        // Both should be cached independently
        assert!(cache.get(&cylinder, 0.01, 1.0).is_some());
        assert!(cache.get(&sphere, 0.01, 1.0).is_some());

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = GeometryCache::new();

        let cylinder = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });

        cache.insert(&cylinder, 0.01, 1.0, Triangulation::new());
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&cylinder, 0.01, 1.0).is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = GeometryCache::new();

        let cylinder = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });

        // 1 miss
        cache.get(&cylinder, 0.01, 1.0);

        cache.insert(&cylinder, 0.01, 1.0, Triangulation::new());

        // 3 hits
        cache.get(&cylinder, 0.01, 1.0);
        cache.get(&cylinder, 0.01, 1.0);
        cache.get(&cylinder, 0.01, 1.0);

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.75);
    }
}
