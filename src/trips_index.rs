use std::{collections::{HashMap, HashSet, BinaryHeap}, time::Instant};
use gtfs_structures::{Gtfs, Id, StopTime, Trip};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

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
    index: HashMap<(&'a str, &'a str), Vec<DirectTrip<'a>>>
}

impl<'a> TripsIndex<'a> {
    pub fn new(gtfs: &'a Gtfs) -> Self {
        println!("[i] Building primary stop_id -> trips[] index");
        let start = Instant::now();
        let mut singular_trips_index: HashMap<&str, HashSet<&String>> = HashMap::new();

        gtfs.stops.values().for_each(|s| {
            let direct_trips: HashSet<&String> = gtfs.trips
                .values()
                .filter(|t| t.stop_times.iter().any(|st| st.stop.id == s.id))
                .map(|t| &t.id)
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
                    return ()
                }

                if let Some(trips_from) = singular_trips_index.get(from.id()) {
                    if let Some(trips_to) = singular_trips_index.get(to.id()) {
                        let direct_trips: Vec<DirectTrip> = trips_from
                            .intersection(trips_to)
                            .filter_map(|t| {
                                if let Some(trip) = gtfs.get_trip(t).ok() {
                                    if let Some(from_idx) = trip.stop_times.iter().position(|st| st.stop.id == from.id()) {
                                        if let Some(to_idx) = trip.stop_times.iter().position(|st| st.stop.id == to.id()) {
                                            if from_idx < to_idx {
                                                return Some(DirectTrip {
                                                    trip,   
                                                    stop_times: &trip.stop_times[from_idx..to_idx + 1],
                                                })
                                            }
                                        }
                                    };
                                }

                                None
                            }).collect();
                        
                        if direct_trips.len() > 0 {
                            trips_index.insert((from.id(), to.id()), direct_trips);
                        }
                    }   
                }
            });
        });

        println!("[i] Done; Took {} s; Index size is {}", start.elapsed().as_secs(), trips_index.len());
        TripsIndex {
            index: trips_index
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

#[derive(Debug)]
pub struct RouteSegment<'a> {
    pub trip_id: &'a str,
    pub start_stop: &'a str,
    pub end_stop: &'a str,
    pub departure_time: SystemTime,
    pub arrival_time: SystemTime,
    pub duration: Duration,
}

// pub fn find_route<'a>(
//     graph: &'a HashMap<&'a str, HashMap<&'a str, Vec<&'a DirectTrip<'a>>>>,
//     start: &'a str,
//     end: &'a str,
//     current_time: SystemTime,
// ) -> Option<Vec<RouteSegment<'a>>> {
//     let mut heap = BinaryHeap::new();
//     let mut distances = HashMap::new();
//     let mut predecessors = HashMap::<&'a str, (&'a DirectTrip<'a>, SystemTime)>::new();

//     distances.insert(start, 0);
//     heap.push(State { cost: 0, position: start });

//     while let Some(State { cost, position }) = heap.pop() {
//         if position == end {
//             let mut path = Vec::new();
//             let mut step = position;

//             while let Some(&(trip, departure_time)) = predecessors.get(step) {
//                 let start_stop = trip.stop_times.first().unwrap().stop.id();
//                 let end_stop = trip.stop_times.last().unwrap().stop.id();
//                 let duration = Duration::from_secs(trip.get_duration() as u64);
//                 let arrival_time = departure_time + duration;

//                 path.push(RouteSegment {
//                     trip_id: trip.trip.id(),
//                     start_stop,
//                     end_stop,
//                     departure_time,
//                     arrival_time,
//                     duration,
//                 });
//                 step = start_stop; // Move to the next segment
//             }
//             path.reverse();
//             return Some(path);
//         }

//         if let Some(neighbours) = graph.get(position) {
//             for (&neighbour, trips) in neighbours {
//                 for trip in trips {
//                     let trip_departure_time = trip.stop_times.first().unwrap().departure_time.unwrap(); // Simplified handling

//                     // Convert trip departure time to SystemTime, compare with current_time
//                     let trip_departure_systemtime = trip_departure_time.into();
//                     if trip_departure_systemtime >= current_time {
//                         let travel_cost = trip.get_duration() + (trip_departure_systemtime.duration_since(current_time).unwrap_or_else(|_| Duration::from_secs(0)).as_secs() as u32);

//                         let next = State { cost: cost + travel_cost, position: neighbour };
//                         if next.cost < *distances.get(neighbour).unwrap_or(&u32::MAX) {
//                             heap.push(next);
//                             distances.insert(neighbour, next.cost);
//                             predecessors.insert(neighbour, (trip, trip_departure_systemtime));
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     None
// }

#[derive(Copy, Clone, Eq, PartialEq)]
struct State<'a> {
    cost: u32,
    position: &'a str,
}

impl<'a> Ord for State<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl<'a> PartialOrd for State<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}