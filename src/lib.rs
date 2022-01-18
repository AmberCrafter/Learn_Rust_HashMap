use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem;

const INITIAL_NBUCKETS: usize = 1;

pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    items: usize,
}

impl<K, V> HashMap<K ,V> 
{
    pub fn new() -> Self {
        HashMap {
            buckets: Vec::new(),
            items: 0,
        }
    }
}

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    entry: &'a mut (K, V),
}
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    key: K,
    map: &'a mut HashMap<K, V>,
    bucket: usize,
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K: 'a, V: 'a> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> &'a mut V 
    {
        self.map.buckets[self.bucket].push((self.key, value));
        self.map.items += 1;
        &mut self.map.buckets[self.bucket].last_mut().unwrap().1
    }
}

impl<'a, K: 'a, V: 'a> Entry<'a, K, V> {
    pub fn or_insert(self, value: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => {
                e.insert(value)
            }
        }
    }

    pub fn or_insert_with<F>(self, maker: F) -> &'a mut V 
    where
        F: FnOnce() -> V
    {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => {
                e.insert(maker())
            }
        }
    }

    pub fn or_insert_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(Default::default)
        // match self {
        //     Entry::Occupied(e) => e.entry.1,
        //     Entry::Vacant(e) => e.insert(V::default())
        // }
    }

}

impl<K, V> HashMap<K ,V> 
where 
    K: Hash + Eq
{
    fn bucket<Q>(&self, key: &Q) -> usize 
    where 
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % self.buckets.len() as u64) as usize
    }

    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        if self.buckets.is_empty() || self.items > 3 * self.buckets.len() / 4 {
            self.resize();
        }

        let bucket = self.bucket(&key);
        
        for entry in &mut self.buckets[bucket] {
            if entry.0 == key {
                return Entry::Occupied(OccupiedEntry {
                    entry: unsafe {
                        &mut *(entry as *mut _)
                    }
                })
            }
        }

        Entry::Vacant(VacantEntry {
            key,
            map: self,
            bucket
        })
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.buckets.is_empty() || self.items > 3 * self.buckets.len() / 4 {
            self.resize();
        }

        let bucket = self.bucket(&key);
        let bucket = &mut self.buckets[bucket];
        
        self.items += 1;
        for &mut (ref ekey, ref mut evalue) in bucket.iter_mut() {
            if ekey == &key {
                return Some(mem::replace(evalue, value));
            }
        }
        bucket.push((key,value));
        None
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V> 
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let bucket = self.bucket(key);
        self.buckets[bucket]
            .iter()
            .find(|&(ref ekey, _)| ekey.borrow() == key)
            .map(|&(_, ref evalue)| evalue)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool 
    where 
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        self.get(key).is_some()
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V> 
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        let bucket = self.bucket(key);
        let bucket = &mut self.buckets[bucket];
        let index = bucket
            .iter()
            .position(|&(ref ekey, _)| ekey.borrow() == key)?;
        self.items -= 1;
        Some(bucket.swap_remove(index).1) // (key, value).1
    }

    pub fn len(&self) -> usize {
        self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items == 0
    }

    fn resize(&mut self) {
        let target_size = match self.buckets.len() {
            0 => INITIAL_NBUCKETS,
            n => 2*n
        };

        let mut new_buckets = Vec::with_capacity(target_size);
        new_buckets.extend((0..target_size).map(|_| Vec::new()));

        for (key, value) in self.buckets.iter_mut().flat_map(|bucket| bucket.drain(..)) {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let bucket = (hasher.finish() % new_buckets.len() as u64) as usize;
            new_buckets[bucket].push((key, value));
        }

        mem::replace(&mut self.buckets, new_buckets);
    }
}

pub struct Iter<'a, K, V> {
    map: &'a HashMap<K, V>,
    bucket: usize,
    at: usize
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.buckets.get(self.bucket) {
                Some(bucket) => {
                    match bucket.get(self.at) {
                        Some(&(ref ekey, ref evalue)) => {
                            self.at += 1;
                            break Some((ekey, evalue))
                        },
                        None => {
                            self.bucket += 1;
                            self.at = 0;
                            // return self.next();
                            continue;
                        }
                    }
                },
                None => break None,
            }
        }
    }
}

impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        Iter{
            map: self,
            bucket: 0,
            at: 0
        }
    }
}

use std::iter::FromIterator;
impl<K, V> FromIterator<(K, V)> for HashMap<K, V>
where 
    K: Hash + Eq
{
    fn from_iter<T>(iter: T) -> Self 
    where
        T: IntoIterator<Item = (K, V)>
    {
        let mut map = HashMap::new();
        for (k, v) in iter {
            map.insert(k ,v);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get() {
        let mut map = HashMap::new();
        map.insert("foo", 42);
        assert_eq!(map.get(&"foo"), Some(&42));
    }

    #[test]
    fn remove() {
        let mut map = HashMap::new();
        map.insert("foo", "bar");
        assert_eq!(map.get(&"foo"), Some(&"bar"));
        assert_eq!(map.remove(&"foo"), Some("bar"));
    }

    #[test]
    fn len_0() {
        let mut map = HashMap::new();
        assert_eq!(map.len(), 0);
        map.insert("foo", "bar");
    }

    #[test]
    fn len_1() {
        let mut map = HashMap::new();
        map.insert("foo", "bar");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn is_empty() {
        let mut map = HashMap::new();
        assert_eq!(map.is_empty(), true);
        map.insert("foo", "bar");
    }

    #[test]
    fn contains_key() {
        let mut map = HashMap::new();
        map.insert("foo", "bar");
        assert_eq!(map.contains_key(&"foo"), true);
        assert_eq!(map.contains_key(&"bar"), false);
    }

    #[test]
    fn iter() {
        let mut map = HashMap::new();
        map.insert("foo", 41);
        map.insert("bar", 42);
        map.insert("baz", 413);
        map.insert("quox", 4);

        for (&key, &value) in &map {
            match key {
                "foo" => assert_eq!(value, 41),
                "bar" => assert_eq!(value, 42),
                "baz" => assert_eq!(value, 413),
                "quox" => assert_eq!(value, 4),
                _ => unreachable!(),
            }
        }

        assert_eq!((&map).into_iter().count(), 4);
    }

}