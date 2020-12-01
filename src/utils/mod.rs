//! Different utilites useful (or useless) in bot development.

use indexmap::IndexSet;
use rustc_hash::{FxHashSet, FxHasher};
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
	data.into_iter().for_each(|p| {
		if solved.contains(&p) {
			return;
		}
		solved.insert(p.clone());

		let neighbors = range_query(&p);
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
	});
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
			.filter(|p| distance(q, &p) <= epsilon)
			.cloned()
			.collect()
	}
}
