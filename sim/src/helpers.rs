use abstutil::WeightedUsizeChoice;
use control::ControlMap;
use driving::DrivingGoal;
use map_model::{BuildingID, BusRoute, BusStopID, LaneID, Map, RoadID};
use std::collections::{BTreeSet, VecDeque};
use walking::SidewalkSpot;
use {
    BorderSpawnOverTime, CarID, Event, OriginDestination, PedestrianID, RouteID, Scenario,
    SeedParkedCars, Sim, SpawnOverTime, Tick,
};

// Helpers to run the sim
impl Sim {
    // TODO share the helpers for spawning specific parking spots and stuff?

    pub fn run_until_done(&mut self, map: &Map, control_map: &ControlMap, callback: Box<Fn(&Sim)>) {
        let mut benchmark = self.start_benchmark();
        loop {
            self.step(&map, &control_map);
            if self.time.is_multiple_of(Tick::from_minutes(1)) {
                let speed = self.measure_speed(&mut benchmark);
                info!("{0}, speed = {1:.2}x", self.summary(), speed);
            }
            callback(self);
            if self.is_done() {
                break;
            }
        }
    }

    pub fn run_until_expectations_met(
        &mut self,
        map: &Map,
        control_map: &ControlMap,
        all_expectations: Vec<Event>,
        time_limit: Tick,
    ) {
        let mut benchmark = self.start_benchmark();
        let mut expectations = VecDeque::from(all_expectations);
        loop {
            if expectations.is_empty() {
                return;
            }
            for ev in self.step(&map, &control_map).into_iter() {
                if ev == *expectations.front().unwrap() {
                    info!("At {}, met expectation {:?}", self.time, ev);
                    expectations.pop_front();
                    if expectations.is_empty() {
                        return;
                    }
                }
            }
            if self.time.is_multiple_of(Tick::from_minutes(1)) {
                let speed = self.measure_speed(&mut benchmark);
                info!("{0}, speed = {1:.2}x", self.summary(), speed);
            }
            if self.time == time_limit {
                panic!(
                    "Time limit {} hit, but some expectations never met: {:?}",
                    self.time, expectations
                );
            }
        }
    }
}

// Spawning helpers
impl Sim {
    pub fn small_spawn(&mut self, map: &Map) {
        let mut s = Scenario {
            scenario_name: "small_spawn".to_string(),
            map_name: map.get_name().to_string(),
            seed_parked_cars: vec![SeedParkedCars {
                neighborhood: "_everywhere_".to_string(),
                cars_per_building: WeightedUsizeChoice {
                    weights: vec![5, 5],
                },
            }],
            spawn_over_time: vec![SpawnOverTime {
                num_agents: 100,
                start_tick: Tick::zero(),
                stop_tick: Tick::from_seconds(5),
                start_from_neighborhood: "_everywhere_".to_string(),
                goal: OriginDestination::Neighborhood("_everywhere_".to_string()),
            }],
            // If there are no sidewalks/driving lanes at a border, scenario instantiation will
            // just warn and skip them.
            border_spawn_over_time: map
                .all_incoming_borders()
                .into_iter()
                .map(|i| BorderSpawnOverTime {
                    num_peds: 10,
                    num_cars: 10,
                    start_tick: Tick::zero(),
                    stop_tick: Tick::from_seconds(5),
                    start_from_border: i.id,
                    goal: OriginDestination::Neighborhood("_everywhere_".to_string()),
                }).collect(),
        };
        for i in map.all_outgoing_borders() {
            s.spawn_over_time.push(SpawnOverTime {
                num_agents: 10,
                start_tick: Tick::zero(),
                stop_tick: Tick::from_seconds(5),
                start_from_neighborhood: "_everywhere_".to_string(),
                goal: OriginDestination::Border(i.id),
            });
        }
        s.instantiate(self, map);

        for route in map.get_all_bus_routes() {
            self.seed_bus_route(route, map);
        }

        /*self.make_ped_using_bus(
            map,
            LaneID(550),
            LaneID(727),
            RouteID(0),
            map.get_l(LaneID(325)).bus_stops[0].id,
            map.get_l(LaneID(840)).bus_stops[0].id,
        );*/

        // TODO this is introducing nondeterminism, because of slight floating point errors.
        // fragile that this causes it, but true. :\
    }

    pub fn big_spawn(&mut self, map: &Map) {
        Scenario {
            scenario_name: "big_spawn".to_string(),
            map_name: map.get_name().to_string(),
            seed_parked_cars: vec![SeedParkedCars {
                neighborhood: "_everywhere_".to_string(),
                cars_per_building: WeightedUsizeChoice {
                    weights: vec![2, 8],
                },
            }],
            spawn_over_time: vec![SpawnOverTime {
                num_agents: 1000,
                start_tick: Tick::zero(),
                stop_tick: Tick::from_seconds(5),
                start_from_neighborhood: "_everywhere_".to_string(),
                goal: OriginDestination::Neighborhood("_everywhere_".to_string()),
            }],
            border_spawn_over_time: map
                .all_incoming_borders()
                .into_iter()
                .map(|i| BorderSpawnOverTime {
                    num_peds: 100,
                    num_cars: 100,
                    start_tick: Tick::zero(),
                    stop_tick: Tick::from_seconds(5),
                    start_from_border: i.id,
                    goal: OriginDestination::Neighborhood("_everywhere_".to_string()),
                }).collect(),
        }.instantiate(self, map);
    }

    pub fn seed_parked_cars(
        &mut self,
        owner_buildins: &Vec<BuildingID>,
        neighborhoods_roads: &BTreeSet<RoadID>,
        cars_per_building: &WeightedUsizeChoice,
        map: &Map,
    ) {
        self.spawner.seed_parked_cars(
            cars_per_building,
            owner_buildins,
            neighborhoods_roads,
            &mut self.parking_state,
            &mut self.rng,
            map,
        );
    }

    pub fn seed_bus_route(&mut self, route: &BusRoute, map: &Map) -> Vec<CarID> {
        // TODO throw away the events? :(
        let mut events: Vec<Event> = Vec::new();
        self.spawner.seed_bus_route(
            &mut events,
            route,
            &mut self.rng,
            map,
            &mut self.driving_state,
            &mut self.transit_state,
            self.time,
        )
    }

    pub fn seed_specific_parked_cars(
        &mut self,
        lane: LaneID,
        // One owner of many spots, kind of weird, but hey, tests. :D
        owner: BuildingID,
        spots: Vec<usize>,
    ) -> Vec<CarID> {
        self.spawner.seed_specific_parked_cars(
            lane,
            owner,
            spots,
            &mut self.parking_state,
            &mut self.rng,
        )
    }

    pub fn make_ped_using_bus(
        &mut self,
        map: &Map,
        from: BuildingID,
        to: BuildingID,
        route: RouteID,
        stop1: BusStopID,
        stop2: BusStopID,
    ) -> PedestrianID {
        self.spawner.start_trip_using_bus(
            self.time.next(),
            map,
            from,
            to,
            stop1,
            stop2,
            route,
            &mut self.trips_state,
        )
    }

    pub fn spawn_specific_pedestrian(&mut self, from: SidewalkSpot, to: SidewalkSpot) {
        self.spawner
            .start_trip_just_walking(self.time.next(), from, to, &mut self.trips_state);
    }

    pub fn make_ped_using_car(&mut self, map: &Map, car: CarID, to: DrivingGoal) {
        let parked = self.parking_state.lookup_car(car).unwrap().clone();
        let owner = parked.owner.unwrap();
        self.spawner.start_trip_using_parked_car(
            self.time.next(),
            map,
            parked,
            &mut self.parking_state,
            owner,
            to,
            &mut self.trips_state,
        );
    }
}
