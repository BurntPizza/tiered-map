
use std::hash::{Hash, BuildHasher};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;

pub struct TieredMap<'a, K: 'a, V: 'a, H: 'a> {
    parent: Option<&'a TieredMap<'a, K, V, H>>,
    map: HashMap<K, V, H>,
    parent_cap: usize,
    parent_size: usize,
}

macro_rules! tm {
    ($parent:expr, $map:expr, $parent_cap:expr, $parent_size:expr) => {
        TieredMap {
            parent: $parent,
            map: $map,
            parent_cap: $parent_cap,
            parent_size: $parent_size,
        }
    }
}

impl<'a, K: Eq + Hash, V> TieredMap<'a, K, V, RandomState> {
    pub fn new() -> Self {
        tm!(None, HashMap::new(), 0, 0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        tm!(None, HashMap::with_capacity(capacity), 0, 0)
    }
}

impl<'a, K: Eq + Hash, V, H: BuildHasher> TieredMap<'a, K, V, H> {
    pub fn with_hasher(hash_builder: H) -> Self {
        tm!(None, HashMap::with_hasher(hash_builder), 0, 0)
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: H) -> Self {
        tm!(None, HashMap::with_capacity_and_hasher(capacity, hash_builder), 0, 0)
    }

    pub fn hasher(&self) -> &H {
        self.map.hasher()
    }

    pub fn capacity(&self) -> usize {
        self.parent_cap + self.map.capacity()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
    }

    pub fn len(&self) -> usize {
        self.parent_size + self.map.len()
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V> 
        where K: Borrow<Q>,
              Q: Hash + Eq
    {
        self.map.get(k).or_else(|| self.parent.and_then(|parent| parent.get(k)))
    }

    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool 
        where K: Borrow<Q>,
              Q: Hash + Eq
    {
        self.map.contains_key(k) || self.parent.map_or_else(|| false, |parent| parent.contains_key(k))
    }
    
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.map.insert(k, v)
    }

    // TODO: iterators, Debug, Eq, etc.
}

impl<'a, K: Hash + Eq, V, H: BuildHasher + Clone> TieredMap<'a, K, V, H> {
    pub fn new_scope(&self) -> TieredMap<K, V, H> {
        // skip empty tiers
        if let Some(p) = self.parent {
            if self.map.is_empty() {
                return p.new_scope()
            }
        }

        tm!(Some(self), HashMap::with_hasher(self.map.hasher().clone()), self.capacity(), self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::TieredMap;

    #[test]
    fn scopes() {
        let mut tm1 = TieredMap::new();
        {
            let mut tm2 = tm1.new_scope();
            
            tm2.insert("a", 0);
            
            assert_eq!(tm2.get("a"), Some(&0));
            assert_eq!(tm1.get("a"), None);
        }

        tm1.insert("a", 1);
        
        let mut tm2 = tm1.new_scope();
        
        tm2.insert("b", 2);

        assert_eq!(tm2.get("a"), Some(&1));
        
        let mut tm3 = tm2.new_scope();
        
        tm3.insert("a", 3);

        assert_eq!(tm2.get("a"), Some(&1));
        assert_eq!(tm3.get("a"), Some(&3));
    }
}
