use std::{cmp::Ordering, collections::{BTreeSet, HashMap}, sync::Arc};
use gtfs_structures::{Gtfs, Stop};
use trigram::similarity;
use geo::{algorithm::haversine_distance::HaversineDistance, Point};

pub struct StopPlatforms {
    pub stop_name: String,
    pub platforms: Vec<Arc<Stop>>,
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

pub struct StopNamesIndex<'a> {
    pub stop_platforms: HashMap<&'a str, Arc<StopPlatforms>>,
}

impl<'a> StopNamesIndex<'_> {
    pub fn new(gtfs: &Gtfs) -> StopNamesIndex {
        // Make an array of unique stop names. BTreeSet was used to
        // always have the same order of elements in set
        let stop_names = gtfs.stops
            .values()
            .map(|s| s.as_ref().name.as_ref().unwrap().as_str())
            .collect::<BTreeSet<&str>>();
        
        let mut stop_platforms: HashMap<&str, Arc<StopPlatforms>> = HashMap::new();

        // (weight, stop name, vector of all stops / platforms for a given stop name)
        stop_names.iter().for_each(|stop_name| {
            // Get all stop platforms for a given stop name
            let platforms = gtfs.stops
                .values()
                .filter_map(|s| {
                    if s.as_ref().name.as_ref().is_some_and(|n| n.as_str() == *stop_name) {
                        Some((*s).clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            let current_stop_platform = Arc::new(StopPlatforms {
                stop_name: stop_name.to_string(),
                platforms,
            });

            stop_platforms.insert(&stop_name, current_stop_platform.clone());
        });


        StopNamesIndex {
            stop_platforms,
        }
    }

    pub fn search_by_name(&self, query: &str) -> Vec<Arc<StopPlatforms>> {
        // (weight, stop name, vector of all stops / platforms for a given stop name)
        let mut weighted_stop_names: Vec<(f32, Arc<StopPlatforms>)> = self.stop_platforms
            .values()
            .map(|sp| (similarity(&sp.stop_name, query), sp.clone()))
            .collect();

        // Move the result with higher score closer to the beginning of an array
        weighted_stop_names.sort_by(|a, b| b.0.total_cmp(&a.0));
        weighted_stop_names.iter().map(|f| f.1.clone()).collect()
    }

    pub fn find_nearest_stops(&self, longitude: f64, latitude: f64, count: usize) -> Vec<Arc<StopPlatforms>> {
        let location = Point::new(longitude, latitude);

        let mut distances: Vec<(f64, Arc<StopPlatforms>)> = self.stop_platforms
            .values()
            .map(|sp| (sp.distance_to_location(location), sp.clone()))
            .collect();

        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        distances.iter().take(count).map(|(_, sp)| sp.clone()).collect()
    }

    pub fn get_stop_name_from_id(&self, id: &str) -> Option<&str> {
        self.stop_platforms
            .values()
            .find(|sp| sp.platforms.iter().any(|p| p.id == id))
            .map(|sp| sp.stop_name.as_str())
    }
}