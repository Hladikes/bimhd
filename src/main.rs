mod stops_index;
mod trips_index;
mod util;

use std::time::Instant;
use gtfs_structures::{Gtfs, Id};
use stops_index::StopNamesIndex;
use trips_index::TripsIndex;
use util::read_line;

use crate::trips_index::DirectTrip;

fn main() {
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let stops_index = StopNamesIndex::new(&gtfs);
    let trips_index = TripsIndex::new(&gtfs);

    loop {
        print!("from: ");
        let from_stop_name = read_line();
        
        print!("  to: ");
        let to_stop_name = read_line();

        let start = Instant::now();
        
        let from = *stops_index.search_by_name(from_stop_name.as_str()).get(0).unwrap();
        let to = *stops_index.search_by_name(to_stop_name.as_str()).get(0).unwrap();

        let mut all_trips: Vec<&DirectTrip> = vec![];

        from.platforms.iter().for_each(|fp| {
            to.platforms.iter().for_each(|tp| {
                if let Some(trips) = trips_index.get_direct_trips(fp.id(), tp.id()) {
                    all_trips.extend(trips);
                }
            });
        });

        let elapsed = start.elapsed();

        println!("-> Going from {} to {}", from.stop_name, to.stop_name);
        println!("-> Total trips {}", all_trips.len());
        all_trips.iter().for_each(|t| {
            println!(
                "-> Trip {}, duration {} s, route {} -> {:#?}", 
                t.trip.id(), 
                t.get_duration(), 
                gtfs.get_route(&t.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())), 
                t.get_stop_names()
            )
        });
        println!("-> Took {} ms\n", elapsed.as_millis());
    }
}