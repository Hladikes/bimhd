use std::{cmp::Ordering, collections::{BTreeSet, HashMap, HashSet}, sync::Arc, time::{Instant, SystemTime}};
use chrono::{DateTime, Local, Timelike};
use geo::{HaversineDistance, Point};
use gtfs_structures::{Gtfs, Id, Stop, StopTime, Trip};
use serde::Serialize;
use trigram::similarity;

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

#[derive(Serialize)]
pub struct DirectTrip<'a> {
    pub trip: &'a Trip,
    pub stop_times: &'a [StopTime],
}

impl<'a> DirectTrip<'a> {
    pub fn get_stop_names(&self) -> Vec<&str> {
        self.stop_times.iter().map(|st| st.stop.name.as_ref().unwrap().as_str()).collect()
    }

    pub fn get_duration(&self) -> u32 {
        self.stop_times.last().unwrap().arrival_time.unwrap() - self.stop_times.first().unwrap().departure_time.unwrap()
    }

    pub fn get_departure_time(&self) -> u32 {
        self.stop_times.first().unwrap().departure_time.unwrap() % 86400
    }

    pub fn get_arrival_time(&self) -> u32 {
        self.stop_times.last().unwrap().arrival_time.unwrap() % 86400
    }

    pub fn get_real_arrival_time(&self) -> u32 {
        if self.get_arrival_time() < self.get_departure_time() {
            self.get_arrival_time() + 86400
        } else {
            self.get_arrival_time()
        }
    }
}

pub struct TransitIndex<'a> {
    gtfs: &'a Gtfs,
    pub platforms: HashMap<&'a str, Arc<StopPlatforms>>,
    pub direct_trips: HashMap<(&'a str, &'a str), Vec<Arc<DirectTrip<'a>>>>,
    pub distances: HashMap<(&'a str, &'a str), f64>,
    pub stops_graph: HashMap<&'a str, HashMap<&'a str, Vec<Arc<DirectTrip<'a>>>>>,
}

impl<'a> TransitIndex<'a> {
    pub fn new(gtfs: &'a Gtfs) -> Self {
        let mut transit_index = TransitIndex {
            gtfs,
            platforms: Self::build_platforms(gtfs),
            direct_trips: Self::build_direct_trips(gtfs),
            distances: Self::build_distances(gtfs),
            stops_graph: HashMap::new(),
        };

        // Build of an index used for a quick direct trip lookup between two stops
        transit_index.direct_trips.keys().into_iter().for_each(|(from, to)| {
            let first_entry = transit_index.stops_graph.entry(from).or_insert(HashMap::new());
            let second_entry = first_entry.entry(to).or_insert(Vec::new());

            if let Some(trip) = transit_index.direct_trips.get(&(from, to)) {
                second_entry.extend(trip.clone())
            }
        });

        transit_index
    }

    fn build_platforms(gtfs: &'a Gtfs) -> HashMap<&str, Arc<StopPlatforms>> {
        // Make an array of unique stop names. BTreeSet was used to
        // always have the same order of elements in set
        let stop_names = gtfs.stops
            .values()
            .map(|s| s.as_ref().name.as_ref().unwrap().as_str())
            .collect::<BTreeSet<&str>>();
        
        // This map serves both as an index for stop_id -> stop (and its platforms),
        // and as a storage for all grouped stop platforms
        let mut stop_platforms: HashMap<&str, Arc<StopPlatforms>> = HashMap::new();

        stop_names.iter().for_each(|stop_name| {
            // Get all stop platforms for a given stop name
            let platforms: Vec<Arc<Stop>> = gtfs.stops
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

            gtfs.stops.values().for_each(|s| {
                if s.as_ref().name.as_ref().is_some_and(|n| n.as_str() == *stop_name) {
                    stop_platforms.insert(s.id(), current_stop_platform.clone());
                }
            });
        });

        stop_platforms
    }

    fn build_direct_trips(gtfs: &'a Gtfs) -> HashMap<(&str, &str), Vec<Arc<DirectTrip>>> {
        println!("[i] Building primary stop_id -> trips[] index");
        let start = Instant::now();
        
        // First index used for indexing stops and corresponding trips, which do include
        // target stops. This is especially helpful for the build up of a second index
        let mut singular_trips_index: HashMap<&str, HashSet<&str>> = HashMap::new();

        gtfs.stops.values().for_each(|s| {
            let direct_trips: HashSet<&str> = gtfs.trips
                .values()
                .filter_map(|t| t.stop_times.iter().any(|st| st.stop.id() == s.id()).then_some(t.id.as_str()))
                .collect();

            singular_trips_index.insert(s.id(), direct_trips);
        });

        println!("[i] Done; Took {} s", start.elapsed().as_secs());

        println!("[i] Building secondary (stop_id, stop_id) -> trips[] index");
        let start = Instant::now();

        // Secondary index used for quick lookups for direct trips between two stops
        let mut trips_index: HashMap<(&str, &str), Vec<Arc<DirectTrip>>> = HashMap::new();
        
        gtfs.stops.values().for_each(|from| {
            gtfs.stops.values().for_each(|to| {
                if from.id() == to.id() {
                    return ();
                }

                let Some(trips_from) = singular_trips_index.get(from.id()) else {
                    return ();
                };
                
                let Some(trips_to) = singular_trips_index.get(to.id()) else {
                    return ();
                };
                
                let direct_trips: Vec<Arc<DirectTrip>> = trips_from
                    .intersection(trips_to)
                    .filter_map(|t| {
                        let Some(trip) = gtfs.get_trip(t).ok() else {
                            return None;
                        };
                        
                        let Some(from_idx) = trip.stop_times.iter().position(|st| st.stop.id() == from.id()) else {
                            return None;
                        };

                        let Some(to_idx) = trip.stop_times.iter().position(|st| st.stop.id() == to.id()) else {
                            return None;
                        };
                        
                        if from_idx < to_idx {
                            return Some(Arc::new(DirectTrip {
                                trip,   
                                stop_times: &trip.stop_times[from_idx..to_idx + 1],
                            }))
                        }

                        None
                    }).collect();
                
                if direct_trips.len() > 0 {
                    trips_index.insert((from.id(), to.id()), direct_trips);
                }
            });
        });

        println!("[i] Done; Took {} s", start.elapsed().as_secs());

        trips_index
    }

    fn build_distances(gtfs: &'a Gtfs) -> HashMap<(&str, &str), f64> {
        println!("[i] Building stop_id -> stop_id -> distance index");
        let start = Instant::now();

        // Secondary index used for quick lookups for distances between two stops
        let mut distances: HashMap<(&str, &str), f64> = HashMap::new();
        gtfs.stops.values().for_each(|from| {
            gtfs.stops.values().for_each(|to| {
                if from.id() == to.id() || distances.contains_key(&(to.id(), from.id())) || distances.contains_key(&(from.id(), to.id())) {
                    return ();
                }

                let from_location = Point::new(from.longitude.unwrap_or(0.0), from.latitude.unwrap_or(0.0));
                let to_location = Point::new(to.longitude.unwrap_or(0.0), to.latitude.unwrap_or(0.0));
                let distance = from_location.haversine_distance(&to_location);

                distances.insert((to.id(), from.id()), distance);
                distances.insert((from.id(), to.id()), distance);
            });
        });
        println!("[i] Done; Took {} s", start.elapsed().as_secs());
        distances
    }

    pub fn search_by_name(&self, query: &str) -> Vec<Arc<StopPlatforms>> {
        // (weight, stop name, vector of all stops / platforms for a given stop name)
        let mut weighted_stop_names: Vec<(f32, Arc<StopPlatforms>)> = self.platforms
            .values()
            .map(|sp| (similarity(&sp.stop_name, query), sp.clone()))
            .collect();

        // Move the result with higher score closer to the beginning of an array
        weighted_stop_names.sort_by(|a, b| b.0.total_cmp(&a.0));
        weighted_stop_names.iter().map(|f| f.1.clone()).collect()
    }

    pub fn find_nearest_stops(&self, longitude: f64, latitude: f64, count: usize) -> Vec<Arc<StopPlatforms>> {
        let location = Point::new(longitude, latitude);

        let mut distances: Vec<(f64, Arc<StopPlatforms>)> = self.platforms
            .values()
            .map(|sp| (sp.distance_to_location(location), sp.clone()))
            .collect();

        distances.sort_by(|a, b| a.0.total_cmp(&b.0));
        distances.iter().take(count).map(|(_, sp)| sp.clone()).collect()
    }

    pub fn get_direct_trips(&self, from_stop_id: &str, to_stop_id: &str) -> Option<Vec<Arc<DirectTrip>>> {
        self.direct_trips.get(&(from_stop_id, to_stop_id)).cloned()
    }

    pub fn get_stop_name_from_id(&self, id: &str) -> Option<&str> {
        self.platforms
            .values()
            .find(|sp| sp.platforms.iter().any(|p| p.id == id))
            .map(|sp| sp.stop_name.as_str())
    }

    pub fn find_route(
        &self,
        start_platforms: Arc<StopPlatforms>,
        end_platforms: Arc<StopPlatforms>,
        start_time_opt: Option<u32>
    ) -> Option<Vec<Arc<DirectTrip>>> {
        let start_time = start_time_opt.unwrap_or_else(|| {
            let current_time = DateTime::<Local>::from(SystemTime::now()).with_timezone(&Local);
            (current_time.hour() as u32) * 3600 + (current_time.minute() as u32) * 60 + (current_time.second() as u32)
        });
        
        let mut best_arrival_time = u32::MAX;
        let mut best_route: Option<Vec<Arc<DirectTrip>>> = None;
    
        for start_platform in start_platforms.platforms.iter() {
            for end_platform in end_platforms.platforms.iter(){
                if let Some(direct_trips) = self.get_direct_trips(start_platform.id.as_str(), end_platform.id.as_str()) {
                    if let Some(best_trip) = direct_trips.iter().filter(|&trip| trip.get_departure_time() >= start_time)
                        .min_by_key(|&trip| trip.get_real_arrival_time()) {
                        if best_trip.get_real_arrival_time() < best_arrival_time {
                            best_arrival_time = best_trip.get_real_arrival_time();
                            best_route = Some(vec![best_trip.clone()]);
                        }
                    }
                }

                let mut possible_transfers = HashMap::new();
                if let Some(start_trips) = self.stops_graph.get(start_platform.id.as_str()) {
                    for (intermediate_stop, trips_from_start) in start_trips {
                        if let Some(trips_to_end) = self.stops_graph.get(intermediate_stop) {
                            if trips_to_end.contains_key(end_platform.id.as_str()) {
                                possible_transfers.insert(intermediate_stop, (trips_from_start, trips_to_end.get(end_platform.id.as_str()).unwrap()));
                            }
                        }
                    }
                }

                for (_transfer_stop, (trips_from_start, trips_to_end)) in possible_transfers {
                    for trip_to_transfer in trips_from_start {
                        if trip_to_transfer.get_departure_time() >= start_time {
                            for trip_from_transfer in trips_to_end {
                                if trip_from_transfer.get_departure_time() >= trip_to_transfer.get_real_arrival_time() {
                                    let arrival_time = trip_from_transfer.get_real_arrival_time();
                                    if arrival_time < best_arrival_time {
                                        best_arrival_time = arrival_time;
                                        best_route = Some(vec![trip_to_transfer.clone(), trip_from_transfer.clone()]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    
        best_route
    }
}