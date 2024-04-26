mod stops_index;
mod trips_index;
mod util;

use std::{collections::HashSet, time::Instant};
use gtfs_structures::{Gtfs, Id, Trip};
use stops_index::StopNamesIndex;
use trips_index::TripsIndex;
use util::read_line;

fn main() {
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let stops_index = StopNamesIndex::new(&gtfs);
    let trips_index = TripsIndex::new(&gtfs);
    
    loop {
        print!("from > ");
        let from = read_line();
        
        print!("to > ");
        let to = read_line();

        let start = Instant::now();
        
        let from_platforms = stops_index.search_by_name(from.as_str()).get(0).unwrap().1;
        let to_platforms = stops_index.search_by_name(to.as_str()).get(0).unwrap().1;

        let mut all_trips: Vec<&Trip> = vec![];

        from_platforms.iter().for_each(|fp| {
            to_platforms.iter().for_each(|tp| {
                if let Some(trips) = trips_index.get_direct_trips(fp.id(), tp.id()) {
                    all_trips.extend(trips);
                }
            });
        });

        let elapsed = start.elapsed();

        let possible_routes: HashSet<String> = all_trips
            .iter()
            .map(|t| {
                let route = gtfs.get_route(t.route_id.as_str()).unwrap();
                route.short_name.clone().unwrap().clone()
            })
            .collect();

        println!(
            "total trips {}, {:?}, {:?}", 
            all_trips.len(), 
            possible_routes, 
            all_trips.iter().map(|t| t.id()).collect::<Vec<&str>>()
        );
        
        println!("Took {} ms\n-", elapsed.as_millis());
    }
}