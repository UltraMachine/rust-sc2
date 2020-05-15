use std::{collections::HashSet, hash::Hash};

pub fn dbscan<P, F>(data: &[P], range_query: F, min_points: usize) -> (Vec<Vec<P>>, HashSet<P>)
where
	P: Eq + Hash + Clone,
	F: Fn(&P) -> Vec<P>,
{
	let mut c = 0;
	let mut clusters = Vec::<Vec<P>>::new();
	let mut noise = HashSet::<P>::new();
	let mut solved = HashSet::<P>::new();
	data.iter().for_each(|p| {
		if !solved.contains(&p) {
			let neighbors = range_query(&p);
			if neighbors.len() < min_points {
				noise.insert(p.clone());
				solved.insert(p.clone());
			} else {
				match clusters.get_mut(c) {
					Some(cluster) => cluster.push(p.clone()),
					None => clusters.push(vec![p.clone()]),
				}
				let mut seed_set = neighbors.into_iter().collect::<HashSet<P>>();
				seed_set.insert(p.clone());

				while !seed_set.is_empty() {
					seed_set.clone().into_iter().for_each(|q| {
						if noise.remove(&q) {
							clusters.get_mut(c).unwrap().push(q.clone());
						} else if !solved.contains(&q) {
							clusters.get_mut(c).unwrap().push(q.clone());
							solved.insert(q.clone());
							let neighbors = range_query(&q);
							if neighbors.len() >= min_points {
								neighbors.iter().for_each(|n| {
									seed_set.insert(n.clone());
								});
							}
						}
						seed_set.remove(&q);
					});
				}
				c += 1;
			}
		}
	});
	(clusters, noise)
}

pub fn range_query<'r, P, D: 'r, F: 'r>(data: &'r [P], distance: F, epsilon: D) -> impl Fn(&P) -> Vec<P> + 'r
where
	P: Clone,
	D: PartialOrd,
	F: Fn(&P, &P) -> D,
{
	move |q: &P| {
		data.iter()
			.filter(|p| distance(q, &p) <= epsilon)
			.cloned()
			.collect::<Vec<P>>()
	}
}
