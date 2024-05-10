// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use samples_protobuf_data_access::module::managed_subscribe::v1::managed_subscribe_callback_server::ManagedSubscribeCallback;
use samples_protobuf_data_access::module::managed_subscribe::v1::{
    CallbackPayload, TopicManagementRequest, TopicManagementResponse,
};

use digital_twin_model::{sdv_v1 as sdv, Metadata};
use log::{debug, info, warn};
use paho_mqtt as mqtt;
use parking_lot::RwLock;
use samples_common::constants::constraint_type;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tokio::sync::{mpsc, watch};
use tokio::time::{sleep, Duration};
use tonic::{Request, Response, Status};

const MQTT_CLIENT_ID: &str = "Speedometer_mood";

#[derive(Clone, Debug)]
pub struct VehicleData {
    pub speed: watch::Receiver<sdv::vehicle::vehicle_speed::TYPE>,
    pub mileage: watch::Receiver<sdv::vehicle::vehicle_mileage::TYPE>,
    pub gear: watch::Receiver<sdv::vehicle::vehicle_gear::TYPE>,
    pub fuel: watch::Receiver<sdv::vehicle::vehicle_fuel::TYPE>,
    pub rpm: watch::Receiver<sdv::vehicle::vehicle_rpm::TYPE>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Property {
    #[serde(rename = "VehicleSpeed")]
    vehicle_speed: sdv::vehicle::vehicle_speed::TYPE,
    #[serde(rename = "$metadata")]
    speed_metadata: Metadata,
    #[serde(rename = "VehicleMileage")]
    vehicle_mileage: sdv::vehicle::vehicle_mileage::TYPE,
    #[serde(rename = "$metadata")]
    mileage_metadata: Metadata,
    #[serde(rename = "VehicleGear")]
    vehicle_gear: sdv::vehicle::vehicle_gear::TYPE,
    #[serde(rename = "$metadata")]
    gear_metadata: Metadata,
    #[serde(rename = "VehicleFuel")]
    vehicle_fuel: sdv::vehicle::vehicle_fuel::TYPE,
    #[serde(rename = "$metadata")]
    fuel_metadata: Metadata,
    #[serde(rename = "VehicleRpm")]
    vehicle_rpm: sdv::vehicle::vehicle_rpm::TYPE,
    #[serde(rename = "$metadata")]
    rpm_metadata: Metadata,
}

/// Actions that are returned from the Pub Sub Service.
#[derive(Clone, EnumString, Eq, Display, Debug, PartialEq)]
pub enum ProviderAction {
    #[strum(serialize = "PUBLISH")]
    Publish,

    #[strum(serialize = "STOP_PUBLISH")]
    StopPublish,
}

#[derive(Debug)]
pub struct TopicInfo {
    topic: String,
    stop_channel: mpsc::Sender<bool>,
}

#[derive(Debug)]
pub struct ProviderImpl {
    pub data_stream: VehicleData,
    pub min_interval_ms: u64,
    entity_map: Arc<RwLock<HashMap<String, Vec<TopicInfo>>>>,
}

/// Create the JSON for the vehicle speed property.
///
/// # Arguments
/// * `vehicle_speed` - The vehicle speed value.
fn create_property_json(data: &VehicleData) -> String {
    let speed_metadata = Metadata { model: sdv::vehicle::vehicle_speed::ID.to_string() };
    let mileage_metadata = Metadata { model: sdv::vehicle::vehicle_mileage::ID.to_string() };
    let gear_metadata = Metadata { model: sdv::vehicle::vehicle_gear::ID.to_string() };
    let fuel_metadata = Metadata { model: sdv::vehicle::vehicle_fuel::ID.to_string() };
    let rpm_metadata = Metadata { model: sdv::vehicle::vehicle_rpm::ID.to_string() };

    let property: Property = Property { vehicle_speed: *data.speed.borrow(), speed_metadata,
                                        vehicle_mileage: *data.mileage.borrow(), mileage_metadata,
                                        vehicle_gear: *data.gear.borrow(), gear_metadata,
                                        vehicle_fuel: *data.fuel.borrow(), fuel_metadata,
                                        vehicle_rpm: *data.rpm.borrow(), rpm_metadata };

    serde_json::to_string(&property).unwrap()
}

/// Establish a connection to a MQTT broker.
///
/// # Arguments
/// `broker_uri` - The MQTT broker's URI.
fn connect_to_broker(broker_uri: &str) -> mqtt::Client {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(broker_uri)
        .client_id(MQTT_CLIENT_ID.to_string())
        .finalize();

    let client = mqtt::Client::new(create_opts)
        .map_err(|err| format!("Failed to create the client due to '{err:?}'")).unwrap();

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(true)
        .finalize();

    let _connect_response =
        client.connect(conn_opts).map_err(|err| format!("Failed to connect due to '{err:?}"));

    return client;
}

impl ProviderImpl {
    /// Initializes provider with entities relevant to itself.
    ///
    /// # Arguments
    /// * `data_stream` - Receiver for data stream for entity.
    /// * `min_interval_ms` - The frequency of the data coming over the data stream.
    pub fn new(data_stream: VehicleData, min_interval_ms: u64) -> Self {
        // Initialize entity map.
        let mut entity_map = HashMap::new();

        // Insert entry for entity id's associated with provider.
        entity_map.insert(sdv::vehicle::vehicle_speed::ID.to_string(), Vec::new());
        entity_map.insert(sdv::vehicle::vehicle_mileage::ID.to_string(), Vec::new());
        entity_map.insert(sdv::vehicle::vehicle_gear::ID.to_string(), Vec::new());
        entity_map.insert(sdv::vehicle::vehicle_fuel::ID.to_string(), Vec::new());
        entity_map.insert(sdv::vehicle::vehicle_rpm::ID.to_string(), Vec::new());

        // Create new instance.
        ProviderImpl { data_stream, min_interval_ms, entity_map: Arc::new(RwLock::new(entity_map)) }
    }

    /// Handles the 'PUBLISH' action from the callback.
    ///
    /// # Arguments
    /// `payload` - Payload sent with the 'PUBLISH' action.
    pub fn handle_publish_action(&self, payload: CallbackPayload) {
        // Get payload information.
        let topic = payload.topic;
        let constraints = payload.constraints;
        let min_interval_ms = self.min_interval_ms;

        // This should not be empty.
        let subscription_info = payload.subscription_info.unwrap();

        // Create stop publish channel.
        let (sender, mut reciever) = mpsc::channel(10);

        // Create topic info.
        let topic_info = TopicInfo { topic: topic.clone(), stop_channel: sender };

        // Record new topic in entity map.
        {
            let mut entity_lock = self.entity_map.write();
            let get_result = entity_lock.get_mut(&payload.entity_id);
            get_result.unwrap().push(topic_info);
        }

        let data_stream = self.data_stream.clone();

        // Start thread for new topic.
        tokio::spawn(async move {
            // Get constraints information.
            let mut frequency_ms = min_interval_ms;

            for constraint in constraints {
                if constraint.r#type == *constraint_type::FREQUENCY_MS {
                    frequency_ms = u64::from_str(&constraint.value).unwrap();
                };
            }

            let broker_uri = subscription_info.uri.clone();
            let client = connect_to_broker(&broker_uri);

            if !client.is_connected() {
                warn!("Failed to conenct for topic '{topic}' on broker {broker_uri}");
                return;
            }

            loop {
                // See if we need to shutdown.
                if reciever.try_recv() == Err(mpsc::error::TryRecvError::Disconnected) {
                    info!("Shutdown thread for {topic}.");

                    if let Err(err) = client.disconnect(None) {
                        warn!("Failed to disconnect from topic '{topic}' on broker {broker_uri} due to {err:?}");
                    }

                    return;
                }

                // Get data from stream at the current instant.
                let content = create_property_json(&data_stream);

                // Publish message to broker.
                info!(
                    "Publish to {topic} for {}, {}, {}, {}, {} with value {content}",
                    sdv::vehicle::vehicle_speed::NAME,
                    sdv::vehicle::vehicle_mileage::NAME,
                    sdv::vehicle::vehicle_gear::NAME,
                    sdv::vehicle::vehicle_fuel::NAME,
                    sdv::vehicle::vehicle_rpm::NAME
                );

                let msg = mqtt::Message::new(&topic, content, mqtt::types::QOS_1);
                if let Err(err) = client.publish(msg) {
                    warn!("Publish failed due to '{err:?}'");
                    break;
                }

                debug!("Completed publish to {topic}.");

                // Sleep for requested amount of time.
                sleep(Duration::from_millis(frequency_ms)).await;
            }
        });
    }

    /// Handles the 'STOP_PUBLISH' action from the callback.
    ///
    /// # Arguments
    /// `payload` - Payload sent with the 'STOP_PUBLISH' action.
    pub fn handle_stop_publish_action(&self, payload: CallbackPayload) {
        let topic_info: TopicInfo;

        let mut entity_lock = self.entity_map.write();
        let get_result = entity_lock.get_mut(&payload.entity_id);

        let topics = get_result.unwrap();

        // Check to see if topic exists.
        if let Some(index) = topics.iter_mut().position(|t| t.topic == payload.topic) {
            // Remove topic.
            topic_info = topics.swap_remove(index);

            // Stop publishing to removed topic.
            drop(topic_info.stop_channel);
        } else {
            warn!("No topic found matching {}", payload.topic);
        }
    }
}

#[tonic::async_trait]
impl ManagedSubscribeCallback for ProviderImpl {
    /// Callback for a provider, will process a provider action.
    ///
    /// # Arguments
    /// * `request` - The request with the action and associated payload.
    async fn topic_management_cb(
        &self,
        request: Request<TopicManagementRequest>,
    ) -> Result<Response<TopicManagementResponse>, Status> {
        let inner = request.into_inner();
        let action = inner.action;
        let payload = inner.payload.unwrap();

        let provider_action = ProviderAction::from_str(&action).unwrap();

        match provider_action {
            ProviderAction::Publish => Self::handle_publish_action(self, payload),
            ProviderAction::StopPublish => Self::handle_stop_publish_action(self, payload),
        }

        Ok(Response::new(TopicManagementResponse {}))
    }
}
