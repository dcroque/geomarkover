use rsparse::data::Sprs;

use crate::data_reader::*;

#[derive(Debug)]
pub enum Value {
    Known(f64),
    Unknown,
}

#[derive(Debug)]
pub struct MarkovNode {
    id: u64,
    street_start: Intersection,
    street_end: Intersection,
    street_data: Street,
    traffic_data: Option<TrafficFlow>,
    transitions: Vec<MarkovTransition>,
}

#[derive(Debug)]
pub struct TrafficFlow {
    estimated_travel_time: Value,
    estimated_average_speed: Value,
    estimated_density: Value,
    normalized_travel_time: Value,
}

#[derive(Debug)]
pub struct MarkovTransition {
    id_to: u64,
    base_probability: Value,
    corrected_probability: Value,
}

#[derive(Debug)]
pub struct TransitionMatrix {
    matrix: Sprs,
}

pub enum TrafficDataSource {
    GoogleRoutes,
    OpenStreetMap,
    NoSource,
}

enum DensityModel {
    Salman2018,
    Wang2013,
}

struct DensityCalculationInputs {
    model: DensityModel,
    current_speed: Option<f64>,
    freeflow_speed: Option<f64>,
    stop_and_go_speed: Option<f64>,
    transition_density: Option<f64>,
    jam_density: Option<f64>,
    vehicle_count: Option<u32>,
    street_length: Option<f64>,
    street_lane_count: Option<f64>,
    markov_transition: Option<f64>,
}

#[derive(Debug)]
pub struct MarkovChain {
    name: String,
    graph: Vec<MarkovNode>,
}

impl MarkovChain {
    fn new_from_network(
        traffic_data_source: TrafficDataSource,
        network_graph: NetworkData,
    ) -> Self {
        let name = network_graph.name;
        let street_vec = network_graph.edges.clone();

        let mut graph: Vec<MarkovNode> = network_graph
            .edges
            .into_iter()
            .map(|x| {
                let traffic_data = MarkovChain::get_traffic_data(&traffic_data_source, &x);
                MarkovNode {
                    id: x.id,
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
            .collect::<Vec<MarkovNode>>()
            .into_iter()
            .map(|mut x| {
                let adjusted_lanes = match x.street_data.oneway {
                    true => x.street_data.lanes,
                    false => x.street_data.lanes / 2.0,
                };
                x.street_data.lanes = adjusted_lanes;
                for y in street_vec.iter() {
                    let y_start = y.start;
                    let x_end = x.street_data.end;
                    match (y_start, x_end) {
                        (ys, xe) if ys == xe => {
                            x.transitions.push(MarkovTransition {
                                id_to: ys,
                                base_probability: Value::Unknown,
                                corrected_probability: Value::Unknown,
                            });
                        }
                        (_, _) => (),
                    }
                }
                x
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

    fn calculate_density(inputs: DensityCalculationInputs) -> f64 {
        match inputs.model {
            DensityModel::Salman2018 => {
                let v = inputs.vehicle_count.unwrap() as f64;
                let l = inputs.street_length.unwrap();
                let n = inputs.street_lane_count.unwrap();
                let pi = inputs.markov_transition.unwrap();

                v*pi/l*n
            }
            DensityModel::Wang2013 => {
                let v = inputs.current_speed.unwrap();
                let vf = inputs.freeflow_speed.unwrap();
                let vb = inputs.stop_and_go_speed.unwrap();
                let kt = inputs.transition_density.unwrap();

                let theta1 = 0.1612 * kt + 0.0337;
                let theta2 = 0.0093 * kt - 0.0507;

                theta1 * f64::ln(((vf - vb) / ((v - vb).powf(1.0 / theta2))) - 1.0) + kt
            }
            _ => 1.0,
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
        let _ = MarkovChain::new_from_network(TrafficDataSource::OpenStreetMap, nw);
    }
}
