use std::sync::atomic::{AtomicUsize, Ordering};

use crate::data_reader::*;

#[derive(Debug, Clone)]
pub enum Value {
    Known(f64),
    Unknown,
}

impl Value {
    pub fn as_f64(&self) -> f64 {
        match &self {
            Value::Known(v) => *v,
            Value::Unknown => f64::NAN,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Value::Known(v) => write!(f, "{}", v),
            Value::Unknown => write!(f, "U"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarkovNode {
    id: u64,
    id_osm: u64,
    street_start: Intersection,
    street_end: Intersection,
    street_data: Street,
    traffic_data: Option<TrafficFlow>,
    transitions: Vec<MarkovTransition>,
}

#[derive(Debug, Clone)]
pub struct TrafficFlow {
    estimated_travel_time: Value,
    estimated_average_speed: Value,
    estimated_density: Value,
    normalized_travel_time: Value,
}

#[derive(Debug, Clone)]
pub struct MarkovTransition {
    id_to: u64,
    probability: Value,
}

#[derive(Debug)]
pub struct TransitionMatrix {
    dim: usize,
    pub matrix: Vec<Value>,
}

impl TransitionMatrix {
    pub fn new_from_markov_chain(mkv_chain: MarkovChain) -> () {
        let dim = mkv_chain.graph.iter().count();
        let mut matrix: Vec<Value> = Vec::new();
    }
}

pub enum TrafficDataSource {
    GoogleRoutes,
    OpenStreetMap,
    NoSource,
    Unknown,
}

impl std::str::FromStr for TrafficDataSource {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gmaps" => Ok(TrafficDataSource::GoogleRoutes),
            "osm" => Ok(TrafficDataSource::OpenStreetMap),
            _ => Ok(TrafficDataSource::Unknown),
        }
    }
}

#[derive(Debug)]
pub struct MarkovChain {
    name: String,
    graph: Vec<MarkovNode>,
}

static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);
impl MarkovChain {
    pub fn new_from_network(
        traffic_data_source: TrafficDataSource,
        network_graph: NetworkData,
    ) -> Self {
        let name = network_graph.name;

        let mut graph: Vec<MarkovNode> = network_graph
            .edges
            .into_iter()
            .map(|x| {
                let traffic_data = MarkovChain::get_traffic_data(&traffic_data_source, &x);
                MarkovNode {
                    id: NEXT_MONITOR.fetch_add(1, Ordering::Relaxed) as u64,
                    id_osm: x.id,
                    street_start: network_graph
                        .nodes
                        .iter()
                        .find(|i| i.id == x.start)
                        .unwrap()
                        .clone(),
                    street_end: network_graph
                        .nodes
                        .iter()
                        .find(|i| i.id == x.end)
                        .unwrap()
                        .clone(),
                    street_data: x,
                    traffic_data,
                    transitions: Vec::new(),
                }
            })
            .collect();

        let street_vec: Vec<(u64, Intersection, Intersection)> = graph
            .clone()
            .into_iter()
            .map(|x| (x.id, x.street_start, x.street_end))
            .collect();

        graph = graph
            .into_iter()
            .map(|mut x| {
                let adjusted_lanes = match x.street_data.oneway {
                    true => x.street_data.lanes,
                    false => x.street_data.lanes / 2.0,
                };
                x.street_data.lanes = adjusted_lanes;
                for y in street_vec.iter() {
                    let x_start = x.street_data.start;
                    let y_start = y.1.id;
                    let x_end = x.street_data.end;
                    let y_end = y.2.id;
                    match (x_start, y_start, x_end, y_end) {
                        (xs, ys, xe, ye) if xs == ys && xe == ye => {
                            x.transitions.push(MarkovTransition {
                                id_to: x.id,
                                probability: x.traffic_data.clone().unwrap().estimated_travel_time,
                            });
                        }
                        (_, ys, xe, _) if ys == xe => {
                            x.transitions.push(MarkovTransition {
                                id_to: y.0,
                                probability: Value::Unknown,
                            });
                        }
                        (_, _, _, _) => (),
                    }
                }
                x
            })
            .collect();

        let min_travel_time = graph
            .iter()
            .map(|x| {
                x.traffic_data
                    .clone()
                    .unwrap()
                    .estimated_travel_time
                    .as_f64()
            })
            .collect::<Vec<f64>>()
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        graph = graph
            .into_iter()
            .map(|mut mkv_node| {
                let norm_tt = mkv_node
                    .traffic_data
                    .clone()
                    .unwrap()
                    .estimated_travel_time
                    .as_f64()
                    / min_travel_time;
                mkv_node.transitions = mkv_node
                    .transitions
                    .into_iter()
                    .map(|mut t| {
                        t.probability = match t.id_to {
                            id if id == mkv_node.id => Value::Known((norm_tt - 1.0) / norm_tt),
                            _ => t.probability,
                        };
                        t
                    })
                    .collect();
                mkv_node
            })
            .map(|mut mkv_node| {
                let self_transition_prob = mkv_node
                    .transitions
                    .clone()
                    .into_iter()
                    .find(|t| t.id_to == mkv_node.id)
                    .unwrap()
                    .probability
                    .as_f64();

                let num_transitions = mkv_node.transitions.len();

                mkv_node.transitions = mkv_node
                    .transitions
                    .into_iter()
                    .map(|mut t| {
                        t.probability = match t.id_to {
                            id if id == mkv_node.id => t.probability,
                            _ => Value::Known(
                                (1.0 - self_transition_prob) / (num_transitions as f64 - 1.0),
                            ),
                        };
                        t
                    })
                    .collect();
                mkv_node
            })
            .collect();
        MarkovChain { name, graph }
    }

    fn get_traffic_data(source: &TrafficDataSource, street_info: &Street) -> Option<TrafficFlow> {
        match source {
            TrafficDataSource::OpenStreetMap => Some(TrafficFlow {
                estimated_travel_time: Value::Known(
                    (street_info.length / 1000.0) / (street_info.maxspeed as f64),
                ),
                estimated_average_speed: Value::Known(street_info.maxspeed as f64),
                estimated_density: Value::Unknown,
                normalized_travel_time: Value::Unknown,
            }),
            _ => None,
        }
    }
}

mod tests {
    use super::{MarkovChain, TrafficDataSource};
    use crate::data_reader::NetworkData;

    #[test]
    fn new_markov_chain_from_file() {
        let nw =
            NetworkData::new_from_file("jose_mendes".to_string(), "output/jose_mendes".to_string());
        let mkv_chain = MarkovChain::new_from_network(TrafficDataSource::OpenStreetMap, nw);
        for node in mkv_chain.graph {
            println!("NODE ID: {:.?}", node.id);
            println!(
                "TT: {:.?}",
                node.traffic_data.unwrap().estimated_travel_time
            );
            for t in node.transitions {
                println!("{:.?}", t);
            }
        }
    }
}
