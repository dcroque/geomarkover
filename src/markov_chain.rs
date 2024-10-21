use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::data_reader::*;
use crate::google_routes::*;

use futures::future;
use serde::Serialize;
use serde_json;

#[derive(Debug, Serialize, Clone)]
pub enum Value {
    Known(f64),
    Unknown(f64),
}

impl Value {
    pub fn as_f64(&self) -> f64 {
        match &self {
            Value::Known(v) => *v,
            Value::Unknown(_) => f64::NAN,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Value::Known(v) => write!(f, "{}", v),
            Value::Unknown(_) => write!(f, "U"),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct MarkovNode {
    id: u64,
    id_osm: u64,
    street_start: Intersection,
    street_end: Intersection,
    street_data: Street,
    traffic_data: Option<TrafficFlow>,
    transitions: Vec<MarkovTransition>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TrafficFlow {
    estimated_travel_time: Value,
    estimated_average_speed: Value,
    estimated_density: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarkovTransition {
    id_to: u64,
    probability: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct TransitionMatrix {
    dim: usize,
    pub matrix: Vec<(usize, usize, f64)>,
}

impl std::ops::Index<(u64, u64)> for TransitionMatrix {
    type Output = f64;

    fn index(&self, i: (u64, u64)) -> &f64 {
        match &self
            .matrix
            .iter()
            .find(|(n, m, _)| *n == i.0 as usize && *m == i.1 as usize)
        {
            None => &0.0,
            Some((_, _, value)) => value,
        }
    }
}

impl TransitionMatrix {
    pub fn new_from_markov_chain(mkv_chain: &MarkovChain) -> Self {
        let dim = mkv_chain.graph.len();
        let mut matrix: Vec<(usize, usize, f64)> = Vec::new();
        for node in &mkv_chain.graph {
            for t in &node.transitions {
                matrix.push((node.id as usize, t.id_to as usize, t.probability.as_f64()));
            }
        }

        TransitionMatrix { dim, matrix }
    }

    fn to(&self, i: u64) -> Vec<(usize, usize, f64)> {
        self.clone()
            .matrix
            .into_iter()
            .filter(|(_, m, _)| *m == i as usize)
            .collect::<Vec<(usize, usize, f64)>>()
    }

    fn _from(&self, i: u64) -> Vec<(usize, usize, f64)> {
        self.clone()
            .matrix
            .into_iter()
            .filter(|(n, _, _)| *n == i as usize)
            .collect::<Vec<(usize, usize, f64)>>()
    }

    pub fn print(&self) {
        let dim = self.dim;
        let matrix = &self.matrix;
        for i in 0..dim {
            let mut line = vec![0.0; dim];
            for x in matrix {
                match x {
                    (a, b, p) if *a == i => {
                        line[*b] = *p;
                    }
                    _ => (),
                }
            }

            for x in line {
                print!("{}\t", x);
            }
            println!();
        }
    }

    pub fn save_to_file(&self, path: String, data_source_str: String) -> bool {
        let dim = self.dim;
        let matrix = &self.matrix;
        let path = format!("{}/transtition_matrix_{}.csv", path, data_source_str);
        if fs::remove_file(path.clone()).is_ok() {
            println!("Removed previous data from {}", path);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();

        for i in 0..dim {
            let mut line = vec![0.0; dim];
            for x in matrix {
                match x {
                    (a, b, p) if *a == i => {
                        line[*b] = *p;
                    }
                    _ => (),
                }
            }

            let mut line_str = "".to_string();
            for x in line {
                let element = format!("{},", x).to_string();
                line_str = format!("{}{}", line_str, element);
            }
            line_str = format!("{}\n", line_str);
            match file.write_all(line_str.as_bytes()) {
                Ok(_) => (),
                _ => {
                    println!("Failed to save content to file");
                    return false;
                }
            }
        }
        true
    }
}

pub enum TrafficDataSource {
    GoogleRoutes(GoogleMapsHandler),
    OpenStreetMap,
    NoSource,
    Unknown,
}

impl TrafficDataSource {
    pub async fn from_str(s: &str) -> Self {
        match s {
            "gmaps" => TrafficDataSource::GoogleRoutes(
                GoogleMapsHandler::new("insert_key_here".to_string()).await,
            ),
            "osm" => TrafficDataSource::OpenStreetMap,
            _ => TrafficDataSource::Unknown,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MarkovChain {
    name: String,
    graph: Vec<MarkovNode>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl MarkovChain {
    pub async fn new_from_network(
        traffic_data_source: TrafficDataSource,
        network_graph: NetworkData,
    ) -> Self {
        let name = network_graph.name;

        let mut graph: Vec<MarkovNode> =
            future::join_all(network_graph.edges.into_iter().map(|x| async {
                MarkovNode {
                    id: ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u64,
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
                    traffic_data: None,
                    transitions: Vec::new(),
                }
            }))
            .await;

        let street_vec: Vec<(u64, Intersection, Intersection)> = graph
            .clone()
            .into_iter()
            .map(|x| (x.id, x.street_start, x.street_end))
            .collect();

        graph = future::join_all(graph.into_iter().map(|mut x| async {
            x.traffic_data = MarkovChain::get_traffic_data(
                &traffic_data_source,
                &x.street_data,
                &x.street_start,
                &x.street_end,
            )
            .await;
            x
        }))
        .await;

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
                                probability: Value::Unknown(0.0),
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

    async fn get_traffic_data(
        source: &TrafficDataSource,
        street_info: &Street,
        street_start: &Intersection,
        street_end: &Intersection,
    ) -> Option<TrafficFlow> {
        match source {
            TrafficDataSource::OpenStreetMap => Some(TrafficFlow {
                estimated_travel_time: Value::Known(
                    (street_info.length / 1000.0) / (street_info.maxspeed as f64),
                ),
                estimated_average_speed: Value::Known(street_info.maxspeed as f64),
                estimated_density: Value::Unknown(0.0),
            }),
            TrafficDataSource::GoogleRoutes(handler) => {
                let traffic_data = match handler
                    .directions(
                        (street_start.latitude, street_start.longitude),
                        (street_end.latitude, street_end.longitude),
                    )
                    .await
                {
                    Ok(r) => r,
                    e => e.unwrap(),
                };

                Some(TrafficFlow {
                    estimated_travel_time: Value::Known(traffic_data.estimated_travel_time),
                    estimated_average_speed: Value::Known(traffic_data.estimated_average_speed),
                    estimated_density: Value::Unknown(0.0),
                })
            }
            _ => None,
        }
    }

    fn node(graph: &[MarkovNode], i: u64) -> MarkovNode {
        graph
            .to_owned()
            .clone()
            .into_iter()
            .find(|x| x.id == i)
            .unwrap()
    }

    pub fn calculate_density_from_matrix(&mut self, t_mtx: &TransitionMatrix, vehicle_count: u64) {
        let h_graph = self.graph.clone();

        self.graph = self
            .graph
            .clone()
            .into_iter()
            .map(|mut x| {
                let self_prob = t_mtx[(x.street_start.id, x.street_end.id)];
                let other_prob = t_mtx.to(x.id);
                let mut density = MarkovChain::calculate_density_parcel(
                    vehicle_count,
                    self_prob,
                    x.street_data.length,
                    x.street_data.lanes,
                );
                for (from, _, prob) in other_prob {
                    let node = MarkovChain::node(&h_graph, from as u64);
                    density += MarkovChain::calculate_density_parcel(
                        vehicle_count,
                        prob,
                        node.street_data.length,
                        node.street_data.lanes,
                    );
                }
                x.traffic_data = Some(TrafficFlow {
                    estimated_travel_time: x.traffic_data.clone().unwrap().estimated_travel_time,
                    estimated_average_speed: x
                        .traffic_data
                        .clone()
                        .unwrap()
                        .estimated_average_speed,
                    estimated_density: Value::Known(density),
                });
                x
            })
            .collect::<Vec<MarkovNode>>();
    }

    fn calculate_density_parcel(v: u64, prob: f64, l: f64, n: f64) -> f64 {
        (v as f64 * prob) / (l * n)
    }

    pub fn save_data(&self, path: String, data_source_str: String) -> bool {
        let output_str: String = match serde_json::to_string_pretty(&self) {
            Ok(v) => v,
            _ => return false,
        };

        let path = format!("{}/markov_chain_{}.json", path, data_source_str);

        let mut file = File::create(path).unwrap();
        match file.write_all(output_str.as_bytes()) {
            Ok(_) => true,
            _ => {
                println!("Failed to save content to file");
                false
            }
        }
    }
}

mod tests {
    #[actix_rt::test]
    async fn new_markov_chain_from_file() {
        let nw = crate::data_reader::NetworkData::new_from_file(
            "jose_mendes".to_string(),
            "output/jose_mendes".to_string(),
        );
        let mkv_chain = super::MarkovChain::new_from_network(
            super::TrafficDataSource::from_str("osm").await,
            nw,
        )
        .await;
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
