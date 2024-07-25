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
pub static mut g_vehicle_fuel: i32 = 0;
pub static mut g_vehicle_rpm: i32 = 0;
pub static mut g_vehicle_wp_fl: i32 = 11;
pub static mut g_vehicle_wp_fr: i32 = 12;
pub static mut g_vehicle_wp_rl: i32 = 21;
pub static mut g_vehicle_wp_rr: i32 = 22;

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
        name: sdv::vehicle_v2::vehicle_speed::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_speed::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_speed::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_mileage_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_mileage::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_mileage::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_mileage::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_gear_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_gear::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_gear::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_gear::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_fuel_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_fuel::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_fuel::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_fuel::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_rpm_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_rpm::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_rpm::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_rpm::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_wheel_pressure_fl_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_wheel_pressure_fl::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_wheel_pressure_fl::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_wheel_pressure_fl::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_wheel_pressure_fr_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_wheel_pressure_fr::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_wheel_pressure_fr::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_wheel_pressure_fr::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_wheel_pressure_rl_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_wheel_pressure_rl::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_wheel_pressure_rl::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_wheel_pressure_rl::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let vehicle_wheel_pressure_rr_entity_access_info = EntityAccessInfo {
        name: sdv::vehicle_v2::vehicle_wheel_pressure_rr::NAME.to_string(),
        id: sdv::vehicle_v2::vehicle_wheel_pressure_rr::ID.to_string(),
        description: sdv::vehicle_v2::vehicle_wheel_pressure_rr::DESCRIPTION.to_string(),
        endpoint_info_list: vec![endpoint_info.clone()],
    };

    let entity_access_info_vec = vec![
        vehicle_speed_entity_access_info,
        vehicle_mileage_entity_access_info,
        vehicle_gear_entity_access_info,
        vehicle_fuel_entity_access_info,
        vehicle_rpm_entity_access_info,
        vehicle_wheel_pressure_fl_entity_access_info,
        vehicle_wheel_pressure_fr_entity_access_info,
        vehicle_wheel_pressure_rl_entity_access_info,
        vehicle_wheel_pressure_rr_entity_access_info,
    ];

    let mut client = InvehicleDigitalTwinClient::connect(invehicle_digital_twin_uri.to_string())
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    let request =
        tonic::Request::new(RegisterRequest { entity_access_info_list: entity_access_info_vec });
    let _response = client.register(request).await?;

    Ok(())
}

/// Start vehicle speed data stream.
///
/// # Arguments
/// 'param' - i32 parameter value
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_speed_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle speed data stream.");
    let (sender, receiver) = watch::channel(10);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_speed) {
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

/// Start vehicle mileage data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_mileage_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle mileage data stream.");
    let (sender, receiver) = watch::channel(9);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_mileage) {
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

/// Start vehicle gear data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_gear_data_stream(min_interval_ms: u64) -> watch::Receiver<i8> {
    debug!("Starting the vehicle gear data stream.");
    let (sender, receiver) = watch::channel(11);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_gear) {
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

/// Start vehicle_fuel data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_fuel_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle fuel data stream.");
    let (sender, receiver) = watch::channel(8);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_fuel) {
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

/// Start vehicle_rpm data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_rpm_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle rpm data stream.");
    let (sender, receiver) = watch::channel(12);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_rpm) {
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

/// Start vehicle_wheel_pressure data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_wheel_pressure_fl_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle wheel pressure data stream.");
    let (sender, receiver) = watch::channel(8);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_wp_fl) {
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

fn start_vehicle_wheel_pressure_fr_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle wheel pressure data stream.");
    let (sender, receiver) = watch::channel(8);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_wp_fr) {
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

fn start_vehicle_wheel_pressure_rl_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle wheel pressure data stream.");
    let (sender, receiver) = watch::channel(8);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_wp_rl) {
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

fn start_vehicle_wheel_pressure_rr_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the vehicle wheel pressure data stream.");
    let (sender, receiver) = watch::channel(8);
    tokio::spawn(async move {
        loop {
            unsafe {
                if let Err(err) = sender.send(g_vehicle_wp_rr) {
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
        let topic = message_mqtt.topic();
        let data = payload.parse::<i32>().unwrap();
        match topic {
            "dashboard/value/speed" => g_vehicle_speed = data,
            "dashboard/value/mileage" => g_vehicle_mileage = data,
            "dashboard/value/gear" => g_vehicle_gear = data as i8,
            "dashboard/value/gas-level" => g_vehicle_fuel = data,
            "dashboard/value/tacho" => g_vehicle_rpm = data,
            "dashboard/value/wheel-pressure-fl" => g_vehicle_wp_fl = data,
            "dashboard/value/wheel-pressure-fr" => g_vehicle_wp_fr = data,
            "dashboard/value/wheel-pressure-rl" => g_vehicle_wp_rl = data,
            "dashboard/value/wheel-pressure-rr" => g_vehicle_wp_rr = data,
            &_ => info!("Not handled topic {topic}!")
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

    // TODO: Add genetic function 'start_data_stream'
    let mut data_stream = provider_impl::VehicleData { speed: start_vehicle_speed_data_stream(interval_ms),
                                                       mileage: start_vehicle_mileage_data_stream(interval_ms),
                                                       gear: start_vehicle_gear_data_stream(interval_ms),
                                                       fuel: start_vehicle_fuel_data_stream(interval_ms),
                                                       rpm: start_vehicle_rpm_data_stream(interval_ms),
                                                       wp_fl: start_vehicle_wheel_pressure_fl_data_stream(interval_ms),
                                                       wp_fr: start_vehicle_wheel_pressure_fr_data_stream(interval_ms),
                                                       wp_rl: start_vehicle_wheel_pressure_rl_data_stream(interval_ms),
                                                       wp_rr: start_vehicle_wheel_pressure_rr_data_stream(interval_ms) };
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

    let topics = settings.subscription_list;
    let broker_uri = settings.broker_uri;

    // Subscribe to topic.
    let can_sub_handle = receive_can_service_updates(&broker_uri, &topics)
        .await
        .map_err(|err| Status::internal(format!("{err:?}")))?;

    server_future.await?;

    signal::ctrl_c().await.expect("Failed to listen for control-c event");

    _ = can_sub_handle.await;

    info!("The Provider has been completed.");

    Ok(())
}
