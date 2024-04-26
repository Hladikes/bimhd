use std::{collections::BTreeSet, sync::Arc, cmp::Ordering};
use gtfs_structures::{Gtfs, Stop};
use trigram::similarity;

pub struct StopNamesIndex {
    pub stops: Vec<(String, Vec<Arc<Stop>>)>,
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
        let stops_name_index: Vec<(String, Vec<Arc<Stop>>)> = stop_names
            .iter()
            .map(|stop_name| {
                // Get all stop platforms for a given stop name
                let target_stops: Vec<Arc<Stop>> = gtfs.stops
                    .values()
                    .filter(|s| s.as_ref().name.as_ref().is_some_and(|n| n == stop_name))
                    .map(|s| (*s).clone())
                    .collect();

                (stop_name.to_string(), target_stops)
            }).collect();

        StopNamesIndex {
            stops: stops_name_index,
        }
    }

    pub fn search_by_name(&self, query: &str) -> Vec<(String, &Vec<Arc<Stop>>)> {
        // (weight, stop name, vector of all stops / platforms for a given stop name)
        let mut weighted_stop_names: Vec<(f32, String, &Vec<Arc<Stop>>)> = self.stops
            .iter()
            .map(|(full_name, stops)| (similarity(&full_name, query), full_name.to_string(), stops))
            .collect();

        // Move the result with higher score closer to the beginning of an array
        weighted_stop_names.sort_by(|a, b| {
            if a.0 > b.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        weighted_stop_names.iter().map(|f| (f.1.to_string(), f.2)).collect()
    }
}