use google_maps::prelude::*;
use google_maps::directions::DepartureTime;

pub struct GoogleMapsHandler {
    client: GoogleMapsClient,
}

#[derive(Debug)]
pub struct RoutesResponse {
    pub distance: f64,
    pub time_secs: f64,
    pub estimated_average_speed: f64,
    pub estimated_travel_time: f64,
}

impl GoogleMapsHandler {
    pub async fn new(gcp_key: String) -> Self {
        let client = GoogleMapsClient::try_new(gcp_key).unwrap();
        GoogleMapsHandler { client }
    }

    pub async fn directions(
        &self,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Result<RoutesResponse, Error> {
        let result = self
            .client
            .directions(
                Location::try_from_f64(from.0, from.1).unwrap(),
                Location::try_from_f64(to.0, to.1).unwrap(),
            )
            .with_travel_mode(TravelMode::Driving)
            .with_departure_time(DepartureTime::Now)
            .execute()
            .await;

        match result {
            Ok(r) => {
                let distance = (r.routes[0].legs[0].distance.value as f64).max(1.0);
                let time_secs = (match &r.routes[0].legs[0].duration_in_traffic {
                    None => r.routes[0].legs[0].duration.value.num_seconds() as f64,
                    Some(v) => v.value.num_seconds() as f64,
                }).max(0.0001);
                let estimated_average_speed = (3.6 * distance) / time_secs;
                let estimated_travel_time = (distance / 1000.0) / estimated_average_speed;
                Ok(RoutesResponse {
                    distance,
                    time_secs,
                    estimated_average_speed,
                    estimated_travel_time,
                })
            }
            e => Err(e.err().unwrap()),
        }
    }
}

mod tests {
    #[actix_rt::test]
    #[ignore]
    async fn test_directions() {
        println!("create client");
        let handler =
            super::GoogleMapsHandler::new("insert_key_here".to_string())
                .await;
        println!("get directions");
        let directions = handler
            .directions((-27.6075094, -48.5478889), (-27.6078129, -48.5477348))
            .await;
        println!("print directions");
        println!("{:.?}", directions);
    }
}
