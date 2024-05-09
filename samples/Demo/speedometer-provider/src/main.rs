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

/// Start the vehicle speed data stream.
///
/// # Arguments
/// `min_interval_ms` - minimum frequency for data stream.
fn start_vehicle_speed_data_stream(min_interval_ms: u64) -> watch::Receiver<i32> {
    debug!("Starting the Provider'vehicle speed data stream.");
    let mut vehicle_speed: i32 = 75;
    let (sender, reciever) = watch::channel(vehicle_speed);
    tokio::spawn(async move {
        let mut is_speed_increasing: bool = true;
        loop {
            debug!(
                "Recording new value for {} of {vehicle_speed}",
                sdv::vehicle::vehicle_speed::ID
            );

            if let Err(err) = sender.send(vehicle_speed) {
                warn!("Failed to get new value due to '{err:?}'");
                break;
            }

            debug!("Completed the publish request");

            // TODO get vehicle data from CAN
            if is_speed_increasing {
                if vehicle_speed == 100 {
                    is_speed_increasing = false;
                    vehicle_speed -= 1;
                } else {
                    vehicle_speed += 1;
                }
            } else if vehicle_speed == 0 {
                is_speed_increasing = true;
                vehicle_speed += 1;
            } else {
                vehicle_speed -= 1;
            }

            sleep(Duration::from_millis(min_interval_ms)).await;
        }
    });

    reciever
}

fn fake_i32_data_stream(value: i32) -> watch::Receiver<i32> {
    debug!("Starting the fake i32 data stream.");
    let mut param: i32 = value;
    let (sender, reciever) = watch::channel(param);
    tokio::spawn(async move {
        loop {
            if let Err(err) = sender.send(param) {
                warn!("Failed to get new value due to '{err:?}'");
                break;
            }

            param = param + 1;
            sleep(Duration::from_millis(1000)).await;
        }
    });

    reciever
}

fn fake_i8_data_stream(value: i8) -> watch::Receiver<i8> {
    debug!("Starting the fake i8 data stream.");
    let mut param: i8 = value;
    let (sender, reciever) = watch::channel(param);
    tokio::spawn(async move {
        loop {
            if let Err(err) = sender.send(param) {
                warn!("Failed to get new value due to '{err:?}'");
                break;
            }

            param = param + 1;
            sleep(Duration::from_millis(1000)).await;
        }
    });

    reciever
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
    let min_interval_ms = 1000; // 1 second
    const SPEED_UPDATE_MS_FLAG: &str = "speed_update_ms=";
    let interval_ms: u64 = std::env::args()
        .find_map(|arg| {
            if arg.contains(SPEED_UPDATE_MS_FLAG) {
                return Some(arg.replace(SPEED_UPDATE_MS_FLAG, ""));
            }

            None
        })
        .unwrap_or_else(|| min_interval_ms.to_string()).parse().unwrap();

    let mut data_stream = provider_impl::VehicleData { speed: start_vehicle_speed_data_stream(interval_ms),
                                                       mileage: fake_i32_data_stream(1000),
                                                       gear: fake_i8_data_stream(1),
                                                       fuel: fake_i8_data_stream(1),
                                                       rpm: fake_i32_data_stream(1000) };
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

    server_future.await?;

    signal::ctrl_c().await.expect("Failed to listen for control-c event");

    info!("The Provider has been completed.");

    Ok(())
}
