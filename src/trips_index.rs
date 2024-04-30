use std::{collections::{HashMap, HashSet, BinaryHeap}, time::Instant};
use gtfs_structures::{Gtfs, Id, StopTime, Trip};
use geo::{Point, algorithm::haversine_distance::HaversineDistance};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Local};

pub struct DirectTrip<'a> {
    pub trip: &'a Trip,
    pub stop_times: &'a [StopTime],
}

impl<'a> DirectTrip<'a> {
    pub fn get_stop_names(&self) -> Vec<String> {
        self.stop_times.iter().map(|st| st.stop.name.clone().unwrap()).collect()
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
}

pub struct TripsIndex<'a> {
    index: HashMap<(&'a str, &'a str), Vec<DirectTrip<'a>>>,
    distances: HashMap<(&'a str, &'a str), f64>,
}

impl<'a> TripsIndex<'a> {
    pub fn new(gtfs: &'a Gtfs) -> Self {
        println!("[i] Building primary stop_id -> trips[] index");
        let start = Instant::now();
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
        let mut trips_index: HashMap<(&str, &str), Vec<DirectTrip>> = HashMap::new();
        
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
                
                let direct_trips: Vec<DirectTrip> = trips_from
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
                            return Some(DirectTrip {
                                trip,   
                                stop_times: &trip.stop_times[from_idx..to_idx + 1],
                            })
                        }

                        None
                    }).collect();
                
                if direct_trips.len() > 0 {
                    trips_index.insert((from.id(), to.id()), direct_trips);
                }
            });
        });

        println!("[i] Done; Took {} s; Index size is {}", start.elapsed().as_secs(), trips_index.len());

        println!("[i] Building distance (stop_id, stop_id) -> f64 index");
        
        let mut distances: HashMap<(&str, &str), f64> = HashMap::new();
        let start = Instant::now();
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

        TripsIndex {
            index: trips_index,
            distances,
        }
    }

    pub fn build_graph(&self) -> HashMap<&str, HashMap<&str, Vec<&DirectTrip>>> {
        let mut graph: HashMap<&str, HashMap<&str, Vec<&DirectTrip>>> = HashMap::new();

        for ((from, to), trips) in &self.index {
            let to_map = graph.entry(from).or_default();
            let trip_list = to_map.entry(to).or_default();
            trip_list.extend(trips.iter().map(|trip| trip));
        }
        graph
    }

    pub fn get_direct_trips(&self, from_stop_id: &'a str, to_stop_id: &'a str) -> Option<&Vec<DirectTrip>> {
        self.index.get(&(from_stop_id, to_stop_id))
    }
}

#[derive(Clone, Eq, PartialEq)]
struct State<'a> {
    cost: u32,
    position: &'a str,
    arrival_time: SystemTime,
}

impl<'a> Ord for State<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
        .then_with(|| self.arrival_time.cmp(&other.arrival_time))
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
    pub departure_time: SystemTime,
    pub arrival_time: SystemTime,
    pub duration: Duration,
}

pub fn find_route(
    graph: &HashMap<&str, HashMap<&str, Vec<&DirectTrip>>>,
    start_stop: &str,
    end_stop: &str,
    start_time: DateTime<Local>,
) -> Option<Vec<RouteSegment>> {
    let mut heap: BinaryHeap<State> = BinaryHeap::new();
    let mut distances: HashMap<&str, u32> = HashMap::new();
    let mut predecessors: HashMap<&str, RouteSegment> = HashMap::new();

    let start_system_time: SystemTime = SystemTime::from(start_time);

    distances.insert(start_stop, 0);
    heap.push(State { cost: 0, position: start_stop, arrival_time: start_system_time });

    while let Some(State { cost, position, arrival_time }) = heap.pop() {
        if position == end_stop {
            let mut path = Vec::new();
            let mut step = position;

            // Reverse trace the path from the destination to the start
            while let Some(segment) = predecessors.get(step) {
                path.push(segment.clone());  // Clone the RouteSegment here
                step = &segment.start_stop;  // Continue tracing back
            }

            path.reverse();
            return Some(path);
        }

        if let Some(neighbours) = graph.get(position) {
            for (&neighbour, trips) in neighbours {
                for trip in trips {
                    let trip_start_time: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(trip.stop_times.first().unwrap().departure_time.unwrap() as u64);
                    if trip_start_time < arrival_time {
                        continue;  // Skip if the trip starts before the current arrival time
                    }

                    let travel_time = trip.get_duration();
                    let new_cost = cost + travel_time;
                    let new_arrival_time = trip_start_time + Duration::from_secs(travel_time as u64);

                    if new_cost < *distances.get(neighbour).unwrap_or(&u32::MAX) {
                        heap.push(State { cost: new_cost, position: neighbour, arrival_time: new_arrival_time });
                        distances.insert(neighbour, new_cost);

                        predecessors.insert(neighbour, RouteSegment {
                            trip_id: trip.trip.id().to_string(),
                            start_stop: position.to_string(),
                            end_stop: neighbour.to_string(),
                            departure_time: trip_start_time,
                            arrival_time: new_arrival_time,
                            duration: Duration::from_secs(travel_time as u64),
                        });
                    }
                }
            }
        }
    }

    None
}