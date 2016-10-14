
use std::ops::Index;
use std::cmp::{PartialEq, Eq};
use std::hash::{Hash, BuildHasher};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::hash_map::{self, RandomState};
use std::fmt::{self, Debug, Formatter};
use std::iter::FromIterator;

pub struct TieredMap<'a, K: 'a, V: 'a, H: 'a = RandomState> {
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

impl<'a, K, V> TieredMap<'a, K, V, RandomState>
    where K: Eq + Hash
{
    pub fn new() -> Self {
        tm!(None, HashMap::new(), 0, 0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        tm!(None, HashMap::with_capacity(capacity), 0, 0)
    }
}

impl<'a, K, V, H> TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher
{
    pub fn with_hasher(hash_builder: H) -> Self {
        tm!(None, HashMap::with_hasher(hash_builder), 0, 0)
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: H) -> Self {
        tm!(None,
            HashMap::with_capacity_and_hasher(capacity, hash_builder),
            0,
            0)
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
        self.map.contains_key(k) ||
        self.parent.map_or_else(|| false, |parent| parent.contains_key(k))
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.map.insert(k, v)
    }

    pub fn iter(&self) -> Iter<K, V, H> {
        Iter {
            map: self,
            iter: self.map.iter(),
        }
    }
}

impl<'a, K, V, H> TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher + Clone
{
    pub fn new_scope(&self) -> TieredMap<K, V, H> {
        // skip empty tiers
        if let Some(p) = self.parent {
            if self.map.is_empty() {
                return p.new_scope();
            }
        }

        tm!(Some(self),
            HashMap::with_hasher(self.map.hasher().clone()),
            self.capacity(),
            self.len())
    }
}

#[derive(Clone)]
pub struct Iter<'a, K: 'a, V: 'a, H: 'a> {
    map: &'a TieredMap<'a, K, V, H>,
    iter: hash_map::Iter<'a, K, V>,
}

impl<'a, K, V, H> Iterator for Iter<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => {
                // current iter is exhausted, move to next tier
                match self.map.parent {
                    None => None, // finished
                    Some(p) => {
                        self.map = p;
                        self.iter = p.map.iter();
                        self.iter.next()
                    }
                }
            }
            s => s,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let l = self.map.parent.map_or(0, |t| t.len()) + self.iter.len();
        (l, Some(l))
    }
}

impl<'a, K, V, H> ExactSizeIterator for Iter<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher
{
    #[inline]
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

impl<'a, K, V, H> IntoIterator for &'a TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, H> Clone for TieredMap<'a, K, V, H>
    where K: Eq + Hash + Clone,
          V: Clone,
          H: BuildHasher + Clone
{
    fn clone(&self) -> Self {
        tm!(self.parent.clone(),
            self.map.clone(),
            self.capacity(),
            self.len())
    }
}

impl<'a, K, V, H> Debug for TieredMap<'a, K, V, H>
    where K: Eq + Hash + Debug,
          V: Debug,
          H: BuildHasher
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a, K, V, H> PartialEq for TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          V: PartialEq,
          H: BuildHasher
{
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() &&
        self.iter().all(|(k, v)| other.get(k).map_or(false, |ov| *v == *ov))
    }
}

impl<'a, K, V, H> Default for TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher + Default
{
    fn default() -> Self {
        Self::with_hasher(Default::default())
    }
}

impl<'a, K, V, H, Q> Index<&'a Q> for TieredMap<'a, K, V, H>
    where K: Eq + Hash + Borrow<Q>,
          H: BuildHasher,
          Q: Eq + Hash
{
    type Output = V;

    #[inline]
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}


impl<'a, K, V, H> Eq for TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          V: Eq,
          H: BuildHasher
{
}

impl<'a, K, V, H> FromIterator<(K, V)> for TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher + Default
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = (K, V)>
    {
        tm!(None, HashMap::from_iter(iter), 0, 0)
    }
}

impl<'a, K, V, H> Extend<(K, V)> for TieredMap<'a, K, V, H>
    where K: Eq + Hash,
          H: BuildHasher
{
    fn extend<T>(&mut self, iter: T)
        where T: IntoIterator<Item = (K, V)>
    {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<'a, K, V, H> Extend<(&'a K, &'a V)> for TieredMap<'a, K, V, H>
    where K: Eq + Hash + Copy,
          V: Copy,
          H: BuildHasher
{
    fn extend<T>(&mut self, iter: T)
        where T: IntoIterator<Item = (&'a K, &'a V)>
    {
        self.extend(iter.into_iter().map(|(&k, &v)| (k, v)));
    }
}

// TODO: quickcheck?
#[cfg(test)]
mod tests {
    use std::collections::{HashSet, HashMap};
    use std::collections::hash_map::RandomState;
    use std::iter::FromIterator;

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

    #[test]
    fn iter() {
        let mut tm = TieredMap::new();
        let mut hm = HashMap::new();

        let entries = &[("a", 0u8), ("d", 3), ("c", 2), ("b", 1)];
        let entries2 = &[("x", 23u8), ("y", 24), ("z", 25)];

        for &(k, v) in entries {
            tm.insert(k, v);
            hm.insert(k, v);
        }

        let mut tm2 = tm.new_scope();

        for &(k, v) in entries2 {
            tm2.insert(k, v);
            hm.insert(k, v);
        }

        assert_eq!(hm.iter().collect::<HashSet<_>>(),
                   tm2.iter().collect::<HashSet<_>>());

        let mut iter1 = hm.iter();
        let mut iter2 = tm2.iter();

        let (mut a, mut b);

        loop {
            assert_eq!(iter1.size_hint(), iter2.size_hint());

            a = iter1.next();
            b = iter2.next();

            if a.is_none() || b.is_none() {
                break;
            }
        }
    }

    #[test]
    fn from_iter() {
        let entries = vec![("a", 0u8), ("d", 3), ("c", 2), ("b", 1), ("z", 4)];
        let len = entries.len();

        // TODO: fix inference
        let tm = TieredMap::<_, _, RandomState>::from_iter(entries);

        assert_eq!(len, tm.len());
    }
}
