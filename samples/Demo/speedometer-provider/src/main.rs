// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

mod provider_impl;

use std::net::SocketAddr;

use digital_twin_model::sdv_v1 as sdv;
use env_logger::{Builder, Target};
use log::{debug, info, warn, LevelFilter};
use samples_common::constants::{digital_twin_operation, digital_twin_protocol};
use samples_common::provider_config;
use samples_common::utils::{retrieve_invehicle_digital_twin_uri, retry_async_based_on_status};
use samples_protobuf_data_access::module::managed_subscribe::v1::managed_subscribe_callback_server::ManagedSubscribeCallbackServer;
use samples_protobuf_data_access::invehicle_digital_twin::v1::invehicle_digital_twin_client::InvehicleDigitalTwinClient;
use samples_protobuf_data_access::invehicle_digital_twin::v1::{
    EndpointInfo, EntityAccessInfo, RegisterRequest,
};
use tokio::sync::watch;
use tokio::signal;
use tokio::time::{sleep, Duration};
use tonic::Status;
use tonic::transport::Server;

use crate::provider_impl::ProviderImpl;

use paho_mqtt as mqtt;
use tokio::task::JoinHandle;
use uuid::Uuid;

const MQTT_CLIENT_ID: &str = "CAN_Speed_updates";
pub static mut g_vehicle_speed: i32 = 75;
pub static mut g_vehicle_mileage: i32 = 0;
pub static mut g_vehicle_gear: i8 = 1;
pub static mut g_vehicle_fuel: i8 = 0;
pub static mut g_vehicle_rpm: i32 = 0;

/// Register the vehicle speed property's endpoint.
///
/// # Arguments
/// * `invehicle_digital_twin_uri` - The In-Vehicle Digital Twin URI.
/// * `provider_uri` - The provider's URI.
async fn register_entities(
    invehicle_digital_twin_uri: &str,
    provider_uri: &str,
) -> Result<(), Status> {
    let endpoint_info = EndpointInfo {
        protocol: digital_twin_protocol::GRPC.to_string(),
        operations: vec![digital_twin_operation::MANAGEDSUBSCRIBE.to_string()],
        uri: provider_uri.to_string(),
        context: "GetSubscriptionInfo".to_string(),
    };

    let vehicle_speed_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle::vehicle_speed::NAME.to_string(),
        id: sdv::vehicle::vehicle_speed::ID.to_string(),
        description: sdv::vehicle::vehicle_speed::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_mileage_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle::vehicle_mileage::NAME.to_string(),
        id: sdv::vehicle::vehicle_mileage::ID.to_string(),
        description: sdv::vehicle::vehicle_mileage::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_gear_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle::vehicle_gear::NAME.to_string(),
        id: sdv::vehicle::vehicle_gear::ID.to_string(),
        description: sdv::vehicle::vehicle_gear::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_fuel_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle::vehicle_fuel::NAME.to_string(),
        id: sdv::vehicle::vehicle_fuel::ID.to_string(),
        description: sdv::vehicle::vehicle_fuel::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_rpm_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle::vehicle_rpm::NAME.to_string(),
        id: sdv::vehicle::vehicle_rpm::ID.to_string(),
        description: sdv::vehicle::vehicle_rpm::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let entity_access_info_vec = vec![
        vehicle_speed_entity_access_info,
        vehicle_mileage_entity_access_info,
        vehicle_gear_entity_access_info,
        vehicle_fuel_entity_access_info,
        vehicle_rpm_entity_access_info
    ];

    let mut client = InvehicleDigitalTwinClient::connect(invehicle_digital_twin_uri.to_string())
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    let request =
        tonic::Request::new(RegisterRequest { entity_access_info_list: entity_access_info_vec });
    let _response = client.register(request).await?;

    Ok(())
}

/// Start i32 data stream.
///
/// # Arguments
/// 'param' - i32 parameter value
/// `min_interval_ms` - minimum frequency for data stream.
/// 'name' - parameter name
fn start_i32_data_stream(param: i32, min_interval_ms: u64, name: &str) -> watch::Receiver<i32> {
    debug!("Starting the {name} data stream.");
    let (sender, receiver) = watch::channel(param);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(param) {
                    warn!("Failed to get new value due to '{err:?}'");
                    break;
                }
            }

            debug!("Completed the publish request");

            sleep(Duration::from_millis(min_interval_ms)).await;
        }
    });

    receiver
}

/// Start i8 data stream.
///
/// # Arguments
/// 'param' - i8 parameter value
/// `min_interval_ms` - minimum frequency for data stream.
/// 'name' - parameter name
fn start_i8_data_stream(param: i8, min_interval_ms: u64, name: &str) -> watch::Receiver<i8> {
    debug!("Starting the {name} data stream.");
    let (sender, receiver) = watch::channel(param);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(param) {
                    warn!("Failed to get new value due to '{err:?}'");
                    break;
                }
            }

            debug!("Completed the publish request");

            sleep(Duration::from_millis(min_interval_ms)).await;
        }
    });

    receiver
}

fn received_can_msg_handler(message_mqtt: paho_mqtt::message::Message)
{
    let payload = std::str::from_utf8(message_mqtt.payload()).unwrap();

    info!("{}", message_mqtt);  //message
    println!("{:02X?}", message_mqtt.payload()); // payload as hex

    unsafe {
        let xxx = message_mqtt.topic();
        match message_mqtt.topic() {
            "dashboard/value/speed" => g_vehicle_speed = payload.parse::<i32>().unwrap(),
            "dashboard/value/mileage" => g_vehicle_mileage = payload.parse::<i32>().unwrap(),
           // "dashboard/value/gear" => g_vehicle_gear = payload.parse::<i8>().unwrap(),
           // "dashboard/value/gas-level" => g_vehicle_fuel = payload.parse::<i8>().unwrap(),
            "dashboard/value/tacho" => g_vehicle_rpm = payload.parse::<i32>().unwrap(),
            &_ => info!("Not handled topic {xxx}!")
        }
    }
}

async fn receive_can_service_updates(
    broker_uri: &str,
    topics: &Vec<String>,
) -> Result<JoinHandle<()>, String> {
    // Create a unique id for the client.
    let client_id = format!("{MQTT_CLIENT_ID}-{}", Uuid::new_v4());

    let create_opts =
        mqtt::CreateOptionsBuilder::new().server_uri(broker_uri).client_id(client_id).finalize();

    let client = mqtt::Client::new(create_opts)
        .map_err(|err| format!("Failed to create MQTT client due to '{err:?}'"))?;

    let receiver = client.start_consuming();

    // Setup task to handle clean shutdown.
    let ctrlc_cli = client.clone();
    tokio::spawn(async move {
        _ = signal::ctrl_c().await;

        // Tells the client to shutdown consuming thread.
        ctrlc_cli.stop_consuming();
    });

    // Last Will and Testament
    let lwt =
        mqtt::MessageBuilder::new().topic("test").payload("Receiver lost connection").finalize();

    let conn_opts = mqtt::ConnectOptionsBuilder::new_v5()
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(false)
        .will_message(lwt)
        .finalize();

    let _connect_response =
        client.connect(conn_opts).map_err(|err| format!("Failed to connect due to '{err:?}"));

    let topics_copy = topics.clone();

    for topic in &topics_copy {
        let mut _subscribe_response = client
            .subscribe(&topic, mqtt::types::QOS_1)
            .map_err(|err| format!("Failed to subscribe to topic {topic} due to '{err:?}'"));
    }

    let sub_handle = tokio::spawn(async move {
        for msg in receiver.iter() {
            if let Some(msg) = msg {
                received_can_msg_handler(msg);
            } else if !client.is_connected() {
                if client.reconnect().is_ok() {
                    for topic in &topics_copy {
                        let mut _subscribe_response = client
                            .subscribe(&topic, mqtt::types::QOS_1)
                            .map_err(|err| {
                                format!("Failed to subscribe to topic {topic} due to '{err:?}'")
                        });
                    }
                } else {
                    break;
                }
            }
        }

        if client.is_connected() {
            debug!("Disconnecting");
            for topic in &topics_copy {
                client.unsubscribe(&topic.as_str()).unwrap();
            }
            client.disconnect(None).unwrap();
        }
    });

    Ok(sub_handle)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging.
    Builder::new().filter(None, LevelFilter::Info).target(Target::Stdout).init();

    info!("The Provider has started.");

    let settings = provider_config::load_settings();

    let provider_authority = settings.provider_authority;
    let provider_uri = format!("http://{provider_authority}"); // Devskim: ignore DS137138

    let invehicle_digital_twin_uri = retrieve_invehicle_digital_twin_uri(
        settings.invehicle_digital_twin_uri,
        settings.chariott_uri,
    )
    .await?;

    // Start mock data stream.
    let min_interval_ms = 300;
    const SPEED_UPDATE_MS_FLAG: &str = "speed_update_ms=";
    let interval_ms: u64 = std::env::args()
        .find_map(|arg| {
            if arg.contains(SPEED_UPDATE_MS_FLAG) {
                return Some(arg.replace(SPEED_UPDATE_MS_FLAG, ""));
            }

            None
        })
        .unwrap_or_else(|| min_interval_ms.to_string()).parse().unwrap();

    // TODO: unsafe block required because use of globals with multiple threads -> to be changed
    unsafe {
    let mut data_stream = provider_impl::VehicleData { speed: start_i32_data_stream(g_vehicle_speed, interval_ms, "speed"),
                                                       mileage: start_i32_data_stream(g_vehicle_mileage, interval_ms, "mileage"),
                                                       gear: start_i8_data_stream(g_vehicle_gear, interval_ms, "gear"),
                                                       fuel: start_i8_data_stream(g_vehicle_fuel, interval_ms, "fuel"),
                                                       rpm: start_i32_data_stream(g_vehicle_rpm, interval_ms, "rpm") };
    info!("MST data_streamhas started.");
    // Setup provider management cb endpoint.
    let provider = ProviderImpl::new(data_stream, min_interval_ms);

    // Start service.
    let addr: SocketAddr = provider_authority.parse()?;
    let server_future =
        Server::builder().add_service(ManagedSubscribeCallbackServer::new(provider)).serve(addr);

    debug!("Sending a register requests to the In-Vehicle Digital Twin Service URI {invehicle_digital_twin_uri}");
    retry_async_based_on_status(30, Duration::from_secs(1), || {
        register_entities(&invehicle_digital_twin_uri, &provider_uri)
    })
    .await?;

    // TODO: "Topic" and "broker" should be made configurable
    let topics = settings.subscription_list;
    let broker_uri = settings.broker_uri;

    // Subscribe to topic.
    let can_sub_handle = receive_can_service_updates(&broker_uri, &topics)
        .await
        .map_err(|err| Status::internal(format!("{err:?}")))?;

    server_future.await?;

    signal::ctrl_c().await.expect("Failed to listen for control-c event");

    _ = can_sub_handle.await;
    }
    info!("The Provider has been completed.");

    Ok(())
}
