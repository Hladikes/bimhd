mod stops_index;

use std::sync::Arc;
use gtfs_structures::{Stop, Gtfs};
use stops_index::StopNamesIndex;

fn main() {
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let stops: Vec<&Arc<Stop>> = gtfs.stops.values().collect();
    
    let index = StopNamesIndex::new(stops);
    index
        .search_by_name("trnauske mito")
        .iter()
        .take(10) // we would most likely only take the first result
        .for_each(|(stop_name, _stop_platforms)| {
            println!("{stop_name}");
        });
}
