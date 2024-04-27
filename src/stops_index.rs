use std::{collections::BTreeSet, sync::Arc};
use gtfs_structures::{Gtfs, Stop};
use trigram::similarity;

pub struct StopPlatforms {
    pub stop_name: String,
    pub platforms: Vec<Arc<Stop>>,
}

pub struct StopNamesIndex {
    pub stop_platforms: Vec<StopPlatforms>,
}

impl StopNamesIndex {
    pub fn new(gtfs: &Gtfs) -> StopNamesIndex {
        // Make an array of unique stop names. BTreeSet was used to
        // always have the same order of elements in set
        let stop_names = gtfs.stops
            .values()
            .map(|s| s.as_ref().name.clone().unwrap())
            .collect::<BTreeSet<String>>();

        // (weight, stop name, vector of all stops / platforms for a given stop name)
        let stop_platforms: Vec<StopPlatforms> = stop_names
            .iter()
            .map(|stop_name| {
                // Get all stop platforms for a given stop name
                StopPlatforms {
                    stop_name: stop_name.clone(),
                    platforms: gtfs.stops
                        .values()
                        .filter(|s| s.as_ref().name.as_ref().is_some_and(|n| n == stop_name))
                        .map(|s| (*s).clone())
                        .collect(),
                }
            }).collect();

        StopNamesIndex {
            stop_platforms,
        }
    }

    pub fn search_by_name(&self, query: &str) -> Vec<&StopPlatforms> {
        // (weight, stop name, vector of all stops / platforms for a given stop name)
        let mut weighted_stop_names: Vec<(f32, &StopPlatforms)> = self.stop_platforms
            .iter()
            .map(|sp| (similarity(&sp.stop_name, query), sp))
            .collect();

        // Move the result with higher score closer to the beginning of an array
        weighted_stop_names.sort_by(|a, b| b.0.total_cmp(&a.0));
        weighted_stop_names.iter().map(|f| f.1).collect()
    }
}