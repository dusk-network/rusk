use alloc::vec::Vec;
use canonical::CanonError;
use canonical_derive::Canon;

#[derive(Clone, Canon, Debug)]
pub struct Collection<K, V> {
    data: Vec<(K, V)>,
}

impl<K: PartialEq, V> Collection<K, V> {
    pub fn get(&self, key: &K) -> Result<Option<&V>, CanonError> {
        Ok(self.data.iter().find(|(x, _)| x == key).map(|(_, y)| y))
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<(), CanonError> {
        self.data.push((key, value));

        Ok(())
    }

    pub fn remove(&mut self, key: &K) -> Result<Option<V>, CanonError> {
        if let Some(index) = self.data.iter().position(|(x, _)| x == key) {
            let (_, val) = self.data.remove(index);

            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

impl<K, V> Default for Collection<K, V> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}
