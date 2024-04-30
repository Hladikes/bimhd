use std::{collections::BTreeSet, sync::Arc, cmp::Ordering};
use gtfs_structures::{Gtfs, Stop};
use trigram::similarity;
use geo::{algorithm::haversine_distance::HaversineDistance, Point};

pub struct StopPlatforms {
    pub stop_name: String,
    pub platforms: Vec<Arc<Stop>>,
}

pub struct StopNamesIndex {
    pub stop_platforms: Vec<StopPlatforms>,
}


impl StopPlatforms {
    pub fn distance_to_location(&self, location: Point<f64>) -> f64 {
        self.platforms.iter()
            .map(|stop| {
                let stop_location = Point::new(stop.longitude.unwrap_or(0.0), stop.latitude.unwrap_or(0.0));
                location.haversine_distance(&stop_location)
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .map(|dist| format!("{:.2}", dist).parse::<f64>().unwrap())
            .unwrap_or(f64::MAX)
    }
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

    pub fn find_nearest_stops(&self, longitude: f64, latitude: f64, count: usize) -> Vec<&StopPlatforms> {
        let location = Point::new(longitude, latitude);

        let mut distances: Vec<(f64, &StopPlatforms)> = self.stop_platforms
            .iter()
            .map(|sp| (sp.distance_to_location(location), sp))
            .collect();

        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        distances.iter().take(count).map(|&(_, sp)| sp).collect()
    }

    pub fn get_stop_name_by_id(&self, stop_id: &str) -> Option<String> {
        for stop_platform in &self.stop_platforms {
            for platform in &stop_platform.platforms {
                if platform.id == stop_id {
                    return Some(stop_platform.stop_name.clone());
                }
            }
        }
        None
    }
}