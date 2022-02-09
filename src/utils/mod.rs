//! Different utilites useful (or useless) in bot development.

use indexmap::IndexSet;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::hash::{BuildHasherDefault, Hash};

type FxIndexSet<T> = IndexSet<T, BuildHasherDefault<FxHasher>>;

/// DBSCAN implementation in Rust.
///
/// Inputs:
/// - `data`: iterable collection of points.
/// - `range_query`: function that should return neighbors of given point.
/// - `min_points`: minimum neighbors required for point to not be marked as noise.
///
/// Returns: (Clusters, Noise).
pub fn dbscan<'a, DT, P, F>(data: DT, range_query: F, min_points: usize) -> (Vec<Vec<P>>, FxHashSet<P>)
where
	DT: IntoIterator<Item = &'a P>,
	P: Eq + Hash + Clone + 'a,
	F: Fn(&P) -> FxIndexSet<P>,
{
	let mut c = 0;
	let mut clusters = Vec::<Vec<P>>::new();
	let mut noise = FxHashSet::<P>::default();
	let mut solved = FxHashSet::<P>::default();
	for p in data {
		if solved.contains(p) {
			continue;
		}
		solved.insert(p.clone());

		let neighbors = range_query(p);
		if neighbors.len() < min_points {
			noise.insert(p.clone());
		} else {
			match clusters.get_mut(c) {
				Some(cluster) => cluster.push(p.clone()),
				None => clusters.push(vec![p.clone()]),
			}

			let mut seed_set = neighbors;
			while let Some(q) = seed_set.pop() {
				if noise.remove(&q) {
					clusters[c].push(q.clone());
				} else if !solved.contains(&q) {
					clusters[c].push(q.clone());
					solved.insert(q.clone());
					let neighbors = range_query(&q);
					if neighbors.len() >= min_points {
						seed_set.extend(neighbors);
					}
				}
			}
			c += 1;
		}
	}
	(clusters, noise)
}

/// Generates `range_query` function for [`dbscan`].
///
/// Takes:
/// - `data`: iterable collection of points (the same data should be passed in [`dbscan`]).
/// - `distance`: function that should returns distance between 2 given points.
/// - `epsilon`: maximum distance between neighbors.
pub fn range_query<'a, DT, P, D, F>(data: DT, distance: F, epsilon: D) -> impl Fn(&P) -> FxIndexSet<P>
where
	DT: IntoIterator<Item = &'a P> + Clone,
	P: Eq + Hash + Clone + 'a,
	D: PartialOrd,
	F: Fn(&P, &P) -> D,
{
	move |q: &P| {
		data.clone()
			.into_iter()
			.filter(|p| distance(q, p) <= epsilon)
			.cloned()
			.collect()
	}
}

#[cfg(feature = "parking_lot")]
use parking_lot::{RwLock, RwLockReadGuard};
#[cfg(not(feature = "parking_lot"))]
use std::sync::{RwLock, RwLockReadGuard};

fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<T> {
	#[cfg(feature = "parking_lot")]
	let reader = lock.read();
	#[cfg(not(feature = "parking_lot"))]
	let reader = lock.read().unwrap();

	reader
}

#[derive(Default)]
pub struct CacheMap<K, V>(RwLock<FxHashMap<K, V>>);
impl<K, V> CacheMap<K, V>
where
	K: Copy + Eq + Hash,
	V: Copy,
{
	pub fn get_or_create<F>(&self, k: &K, f: F) -> V
	where
		F: FnOnce() -> V,
	{
		let lock = read(&self.0);
		if let Some(res) = lock.get(k) {
			*res
		} else {
			drop(lock);

			#[cfg(feature = "parking_lot")]
			let mut lock = self.0.write();
			#[cfg(not(feature = "parking_lot"))]
			let mut lock = self.0.write().unwrap();

			let res = f();
			lock.insert(*k, res);
			res
		}
	}
	pub fn get(&self, k: &K) -> Option<V> {
		read(&self.0).get(k).copied()
	}
}
