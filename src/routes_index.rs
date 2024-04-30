use gtfs_structures::{Gtfs, Route};

pub struct RoutesIndex<'a> {
    pub routes: Vec<&'a Route>,
}

impl<'a> RoutesIndex<'a> {
    pub fn new(gtfs: &'a Gtfs) -> Self {
        let routes = gtfs.routes.values().collect();
        RoutesIndex { routes }
    }

    pub fn get_route_by_id(&self, route_id: &str) -> Option<&Route> {
        self.routes.iter().find(|&r| r.id == route_id).copied()
    }

    pub fn get_route_by_name(&self, route_name: &str) -> Option<&Route> {
        self.routes.iter().find(|&r| r.short_name.as_deref() == Some(route_name)).copied()
    }
}