use std::{collections::{HashMap, HashSet}, time::Instant};
use gtfs_structures::{Gtfs, Id, StopTime, Trip};

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

        self.index.keys().into_iter().for_each(|(from, to)| {
            let first_entry = graph.entry(from).or_insert(HashMap::new());
            let second_entry = first_entry.entry(to).or_insert(Vec::new());

            if let Some(trip) = self.index.get(&(from, to)) {
                second_entry.extend(trip)
            }
        });

        graph
    }

    pub fn get_direct_trips(&self, from_stop_id: &'a str, to_stop_id: &'a str) -> Option<&Vec<DirectTrip>> {
        self.index.get(&(from_stop_id, to_stop_id))
    }
}