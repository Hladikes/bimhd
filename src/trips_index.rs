use std::{collections::{HashMap, HashSet}, time::Instant};
use gtfs_structures::{Gtfs, Id, Trip};

pub struct TripsIndex<'a> {
    index: HashMap<(&'a str, &'a str), Vec<&'a Trip>>
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
        let mut trips_index: HashMap<(&str, &str), Vec<&Trip>> = HashMap::new();
        
        gtfs.stops.values().for_each(|from| {
            gtfs.stops.values().for_each(|to| {
                if from.id() == to.id() {
                    return ()
                }

                if let Some(trips_from) = singular_trips_index.get(from.id()) {
                    if let Some(trips_to) = singular_trips_index.get(to.id()) {
                        let mut trips_intersection: Vec<&Trip> = trips_from
                            .intersection(trips_to)
                            .map(|t| gtfs.get_trip(*t).expect("Could not find a trip for a given id"))
                            .collect();

                        trips_intersection.retain(|t| {
                            let from_idx = t.stop_times
                                .iter()
                                .position(|st| st.stop.id == from.id())
                                .expect("This should not panic");

                            let to_idx = t.stop_times
                                .iter()
                                .position(|st| st.stop.id == to.id())
                                .expect("This should not panic");

                            from_idx < to_idx
                        });
                        
                        if trips_intersection.len() > 0 {
                            trips_index.insert((from.id(), to.id()), trips_intersection);
                        }
                    }   
                }
            });
        });

        println!("[i] Done; Took {} s", start.elapsed().as_secs());
        TripsIndex {
            index: trips_index
        }
    }

    pub fn get_direct_trips(&self, from_stop_id: &'a str, to_stop_id: &'a str) -> Option<&Vec<&Trip>> {
        self.index.get(&(from_stop_id, to_stop_id))
    }
}