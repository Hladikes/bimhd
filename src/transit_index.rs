use std::{cmp::Ordering, collections::{BTreeSet, BinaryHeap, HashMap, HashSet}, sync::Arc, time::{Duration, Instant, SystemTime}};
use chrono::{DateTime, Local, NaiveTime, Timelike};
use geo::{HaversineDistance, Point};
use gtfs_structures::{Gtfs, Id, Stop, StopTime, Trip};
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

pub struct DirectTrip<'a> {
    pub trip: &'a Trip,
    pub stop_times: &'a [StopTime],
}

impl<'a> DirectTrip<'a> {
    pub fn get_stop_names(&self) -> Vec<&str> {
        self.stop_times.iter().map(|st| st.stop.name.as_ref().unwrap().as_str()).collect()
    }

    pub fn get_duration(&self) -> u32 {
        let departure_time = self.stop_times.first()
            .and_then(|t| t.arrival_time)
            .unwrap_or(0);

        let arrival_time = self.stop_times.last()
            .and_then(|t| t.arrival_time)
            .unwrap_or(0);

        arrival_time.abs_diff(departure_time)
    }

    pub fn get_departure_time(&self) -> u32 {
        self.stop_times.first().unwrap().departure_time.unwrap() % 86400
    }

    pub fn get_arrival_time(&self) -> u32 {
        self.stop_times.last().unwrap().arrival_time.unwrap() % 86400
    }
}


#[derive(Clone, Eq, PartialEq)]
struct State<'a> {
    cost: u32,
    position: &'a str,
    arrival_time: u32,
}

impl<'a> Ord for State<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost).then_with(|| self.arrival_time.cmp(&other.arrival_time))
    }
}

impl<'a> PartialOrd for State<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct RouteSegment {
    pub trip_id: String,
    pub start_stop: String,
    pub end_stop: String,
    pub departure_time: NaiveTime,
    pub arrival_time: NaiveTime,
    pub duration: Duration,
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

    pub fn get_direct_trips(&self, from_stop_id: &'a str, to_stop_id: &'a str) -> Option<&Vec<Arc<DirectTrip>>> {
        self.direct_trips.get(&(from_stop_id, to_stop_id))
    }

    pub fn get_stop_name_from_id(&self, id: &str) -> Option<&str> {
        self.platforms
            .values()
            .find(|sp| sp.platforms.iter().any(|p| p.id == id))
            .map(|sp| sp.stop_name.as_str())
    }

    // pub fn find_route(
    //     &self,
    //     start_stop: &'a str,
    //     end_stop: &'a str,
    // ) -> Option<Vec<RouteSegment>> {
    //     let mut heap: BinaryHeap<State> = BinaryHeap::new();
    //     let mut distances: HashMap<&str, u32> = HashMap::new();
    //     let mut predecessors: HashMap<&str, RouteSegment> = HashMap::new();
    //     let start_time: NaiveTime = DateTime::<Local>::from(SystemTime::now()).with_timezone(&Local).time();
        

    //     println!("Starting search from: {:?} to {:?}, at {:?}", self.get_stop_name_from_id(start_stop).unwrap(), self.get_stop_name_from_id(end_stop).unwrap(), start_time);
    
    //     distances.insert(start_stop, 0);
    //     heap.push(State { cost: 0, position: start_stop, arrival_time: start_time });
    
    //     while let Some(current) = heap.pop() {
    //         println!("Processing stop: {:?} with cost {} and arrival at {:?}", self.get_stop_name_from_id(current.position).unwrap(), current.cost, current.arrival_time);
    //         if current.position == end_stop {
    //             println!("Destination reached.");
    //             return Some(Self::construct_path(predecessors, end_stop));
    //         }
    
    //         if let Some(neighbors) = self.stops_graph.get(current.position) {
    //             for (&neighbor, trips) in neighbors {
    //                 Self::process_neighbor_trips(&self, &mut heap, &mut distances, &mut predecessors, trips, &current, neighbor);
    //             }
    //         }
    //     }
    //     None
    // }

    pub fn find_route(
        &self,
        start_stop_id: &'a str,
        end_stop_id: &'a str,
    ) -> Option<Vec<DirectTrip<'a>>> {
        let mut visited = HashSet::new();
        let mut heap = BinaryHeap::new();

        // Initialize with the start time converted to seconds from midnight
        let start_time = DateTime::<Local>::from(SystemTime::now()).with_timezone(&Local).time();
        let start_seconds = (start_time.hour() as u32) * 3600 + (start_time.minute() as u32) * 60 + (start_time.second() as u32);

        heap.push(State {
            cost: 0,
            position: start_stop_id,
            arrival_time: start_seconds,
        });

        while let Some(current) = heap.pop() {
            if current.position == end_stop_id {
                // Backtrack to reconstruct the path from start to end
                return self.reconstruct_path(current.position, &visited);
            }

            // Avoid revisiting nodes
            if visited.contains(&(current.position, current.arrival_time)) {
                continue;
            }
            visited.insert((current.position, current.arrival_time));

            // Explore neighboring stops
            if let Some(trips) = self.stops_graph.get(current.position) {
                for (&next_stop, direct_trips) in trips {
                    for trip in direct_trips {
                        // Ensure the trip starts after the current position's arrival time
                        let trip_start_time = trip.get_departure_time();
                        if trip_start_time >= current.arrival_time {
                            let arrival_time = trip.get_arrival_time();
                            let travel_time = trip.get_duration();

                            heap.push(State {
                                cost: current.cost + travel_time,
                                position: next_stop,
                                arrival_time: arrival_time,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn reconstruct_path(
        &self,
        end_stop: &'a str,
        visited: &HashSet<(&'a str, u32)>
    ) -> Option<Vec<DirectTrip<'a>>> {
        let mut path = Vec::new();
        let mut step = end_stop;

        // Reverse iteration to find the path from end to start
        while let Some((pos, arr_time)) = visited.get(&(step, arr_time)) {
            let trips = self.direct_trips.get(&(pos, step)).unwrap();
            let trip = trips.iter().find(|t| t.get_arrival_time() == *arr_time).unwrap();
            path.push(*trip);
            step = pos;
        }

        path.reverse();
        Some(path)
    }
}

//     fn construct_path(predecessors: HashMap<&str, RouteSegment>, position: &str) -> Vec<RouteSegment> {
//         let mut path = Vec::new();
//         let mut step = position;
    
//         while let Some(segment) = predecessors.get(step) {
//             path.push(segment.clone());
//             step = &segment.start_stop;
//         }
    
//         path.reverse();
//         path
//     }

//     fn process_neighbor_trips(
//         &self,
//         heap: &mut BinaryHeap<State<'a>>,
//         distances: &mut HashMap<&'a str, u32>,
//         predecessors: &mut HashMap<&'a str, RouteSegment>,
//         trips: &[Arc<DirectTrip<'a>>],
//         current: &State<'a>,
//         neighbor: &'a str
//     ) {
//         for trip in trips {
//             let sec_from_midnight = trip.stop_times.first().unwrap().departure_time.unwrap();
    
//             if NaiveTime::from_hms_opt(sec_from_midnight / 3600, (sec_from_midnight % 3600) / 60, sec_from_midnight % 60).is_none() {
//                 //println!("Invalid time found: {}, trip: {}", sec_from_midnight, trip.trip.id);
//                 continue;
//             }
    
//             let trip_start_time: NaiveTime = NaiveTime::from_hms_opt(sec_from_midnight / 3600, (sec_from_midnight % 3600) / 60, sec_from_midnight % 60).unwrap();
                 
//             if trip_start_time >= current.arrival_time {
//                 let travel_time = trip.get_duration();
//                 let new_cost = current.cost + travel_time;
//                 let new_arrival_time = trip_start_time + Duration::from_secs(travel_time as u64);
        
//                 if new_cost < *distances.get(neighbor).unwrap_or(&u32::MAX) {
//                     println!("Better Trip found {},from {:?} to {:?}, new arrival_time: {}", trip.trip.id, self.get_stop_name_from_id(current.position).unwrap(), self.get_stop_name_from_id(neighbor).unwrap(), new_arrival_time);
//                     heap.push(State { cost: new_cost, position: neighbor, arrival_time: new_arrival_time });
//                     distances.insert(neighbor, new_cost);
//                     predecessors.insert(neighbor, RouteSegment {
//                         trip_id: trip.trip.id().to_string(),
//                         start_stop: current.position.to_string(),
//                         end_stop: neighbor.to_string(),
//                         departure_time: trip_start_time,
//                         arrival_time: new_arrival_time,
//                         duration: Duration::from_secs(travel_time as u64),
//                     });
//                 } 
//             } 
//         }
//     }
    
// }