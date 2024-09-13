// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.msg
// SPDX-License-Identifier: MIT

mod dashboard_update;
use crate::dashboard_update::{update_dashboard};

use std::env;

use digital_twin_model::sdv_v1 as sdv;
use env_logger::{Builder, Target};
use log::{debug, info, LevelFilter};
use paho_mqtt as mqtt;
use samples_common::constants::{constraint_type, digital_twin_operation, digital_twin_protocol};
use samples_common::consumer_config;
use samples_common::utils::{
    discover_digital_twin_provider_using_ibeji, retrieve_invehicle_digital_twin_uri,
};
use samples_protobuf_data_access::module::managed_subscribe::v1::managed_subscribe_client::ManagedSubscribeClient;
use samples_protobuf_data_access::module::managed_subscribe::v1::{
    Constraint, SubscriptionInfoRequest, SubscriptionInfoResponse,
};
use tokio::signal;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tonic::{Request, Status};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use std::str::FromStr;

#[derive(Debug,Deserialize,Serialize)]
struct DataPacket {
    VehicleSpeed: sdv::vehicle_v3::vehicle_speed::TYPE,
    VehicleMileage: sdv::vehicle_v3::vehicle_mileage::TYPE,
    VehicleGear: sdv::vehicle_v3::vehicle_gear::TYPE,
    VehicleFuel: sdv::vehicle_v3::vehicle_fuel::TYPE,
    VehicleRpm: sdv::vehicle_v3::vehicle_rpm::TYPE,
    VehicleWheelPressureFL: sdv::vehicle_v3::vehicle_wheel_pressure_fl::TYPE,
    VehicleWheelPressureFR: sdv::vehicle_v3::vehicle_wheel_pressure_fr::TYPE,
    VehicleWheelPressureRL: sdv::vehicle_v3::vehicle_wheel_pressure_rl::TYPE,
    VehicleWheelPressureRR: sdv::vehicle_v3::vehicle_wheel_pressure_rr::TYPE
}

const FREQUENCY_MS_FLAG: &str = "freq_ms=";
const MQTT_CLIENT_ID: &str = "Speedometer_consumer";

const RED_RGB_COLOR: u32 = 0xFF0000;   //    rgb(255, 0, 0)
const GREEN_RGB_COLOR: u32 = 0x008000; //	rgb(0,128,0)

const LED_DELAY_MS_FLAG: &str = "led_delay_ms=";
const LED_COLOR_FLAG: &str = "led_color=";
const LED_COUNT_FLAG: &str = "led_count=";

const MAX_SPEED_FLAG: &str = "max_speed=";
const MAX_SPEED_DEFAULT: i32 = 100;

const BLINK_DELAY_FLAG: &str = "blink_delay=";
const BLINK_DEFAULT_DELAY: u32 = 1;

const BLUEBOLT_MODE: &str = "mode=";
const BLUEBOLT_MODE_OFF: &str = "off";
const BLUEBOLT_MODE_LED: &str = "led";
const BLUEBOLT_MODE_GRADIENT: &str = "gradient";
const BLUEBOLT_MODE_SPEED: &str = "speed";
const BLUEBOLT_MODE_DEFAULT: &str = "";

#[cfg(target_arch = "aarch64")]
pub static mut display_ptr: *mut led_driver::Display = std::ptr::null_mut();
pub static mut max_speed: i32 = MAX_SPEED_DEFAULT;
pub static mut blink_delay: u32 = 3;
pub static mut blink_cur_delay: u32 = 0;
pub static mut blink_color: u32 = 0;

/// Get subscription information from managed subscribe endpoint.
///
/// # Arguments
/// * `managed_subscribe_uri` - The managed subscribe URI.
/// * `constraints` - Constraints for the managed topic.
async fn get_vehicle_subscription_info(
    managed_subscribe_uri: &str,
    constraints: Vec<Constraint>,
) -> Result<SubscriptionInfoResponse, Status> {
    // Create gRPC client.
    let mut client = ManagedSubscribeClient::connect(managed_subscribe_uri.to_string())
        .await
        .map_err(|err| Status::from_error(err.into()))?;

    let request = Request::new(SubscriptionInfoRequest {
        entity_id: sdv::vehicle_v3::vehicle_speed::ID.to_string(),
        constraints,
    });

    let response = client.get_subscription_info(request).await?;

    Ok(response.into_inner())
}

fn received_msg_handler(message_mqtt: paho_mqtt::message::Message)
{
    let payload = std::str::from_utf8(message_mqtt.payload()).unwrap();
    let data: DataPacket = serde_json::from_str(payload).unwrap();

    info!("{}", message_mqtt);  //message
    //println!("{:02X?}", message_mqtt.payload()); // payload as hex
    println!("received {:?}", data); //data extracted from the message

    //Inform dashboard
    send_to_dashboard(data);
}

fn send_to_dashboard(data: DataPacket)
{
    unsafe{
        dashboard_update::current_vehicle_speed = data.VehicleSpeed;
        dashboard_update::current_vehicle_mileage = data.VehicleMileage;
        dashboard_update::current_vehicle_gear = data.VehicleGear;
        dashboard_update::current_vehicle_fuel = data.VehicleFuel;
        dashboard_update::current_vehicle_rpm = data.VehicleRpm;
        dashboard_update::current_vehicle_wheel_pressure_fl = data.VehicleWheelPressureFL;
        dashboard_update::current_vehicle_wheel_pressure_fr = data.VehicleWheelPressureFR;
        dashboard_update::current_vehicle_wheel_pressure_rl = data.VehicleWheelPressureRL;
        dashboard_update::current_vehicle_wheel_pressure_rr = data.VehicleWheelPressureRR;

        #[cfg(target_arch = "aarch64")]
        {
            let color_code_rgb_left = 0x00200000;
            let color_code_rgb_right =  0x00002000;
            let half_max_speed = max_speed / 2;

            if data.VehicleSpeed == 0 {
                // no speed
                (*display_ptr).setAllLedsToRgb(color_code_rgb_right);
            } else if data.VehicleSpeed <= half_max_speed {
                (*display_ptr).setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, 0, (31*data.VehicleSpeed/half_max_speed) as usize);
            } else if data.VehicleSpeed >= max_speed {
                // too fast
                if blink_cur_delay > blink_delay {
                    if blink_color > 0 {
                        blink_color = 0;
                    } else {
                        blink_color = color_code_rgb_left;
                    }
                    blink_cur_delay = 0;
                }
                blink_cur_delay = blink_cur_delay + 1;

                (*display_ptr).setAllLedsToRgb(blink_color);
            } else {
                (*display_ptr).setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, (31*(data.VehicleSpeed-half_max_speed)/half_max_speed) as usize, 31);
            }
        }
    }
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

/// Receive vehicle speed updates.
///
/// # Arguments
/// * `broker_uri` - The broker URI.
/// * `topic` - The topic.
async fn receive_vehicle_data_updates(
    broker_uri: &str,
    topic: &str,
) -> Result<JoinHandle<()>, String> {
    // Create a unique id for the client.
    let client_id = format!("{MQTT_CLIENT_ID}-{}", Uuid::new_v4());

    let create_opts =
        mqtt::CreateOptionsBuilder::new().server_uri(broker_uri).client_id(client_id).finalize();

    let client = mqtt::Client::new(create_opts)
        .map_err(|err| format!("Failed to create MQTT client due to '{err:?}'"))?;

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
        .keep_alive_interval(Duration::from_secs(10))
        .clean_session(false)
        .will_message(lwt)
        .finalize();

    let _connect_response =
        client.connect(conn_opts).map_err(|err| format!("Failed to connect due to '{err:?}"));

    let receiver = client.start_consuming();

    let mut _subscribe_response = client
        .subscribe(topic, mqtt::types::QOS_1)
        .map_err(|err| format!("Failed to subscribe to topic {topic} due to '{err:?}'"));

    // Copy topic for separate thread.
    let topic_string = topic.to_string();

    let sub_handle = tokio::spawn(async move {
        for msg in receiver.iter() {
            if let Some(msg) = msg {

                received_msg_handler(msg);
//                print_type_of(&msg);
            } else if !client.is_connected() {
                if client.reconnect().is_ok() {
                    _subscribe_response = client
                        .subscribe(topic_string.as_str(), mqtt::types::QOS_1)
                        .map_err(|err| {
                            format!("Failed to subscribe to topic {topic_string} due to '{err:?}'")
                        });
                } else {
                    break;
                }
            }
        }

        if client.is_connected() {
            debug!("Disconnecting");
            client.unsubscribe(topic_string.as_str()).unwrap();
            client.disconnect(None).unwrap();
        }
    });

    Ok(sub_handle)
}

fn get_cmd_arg<T: std::str::FromStr>(arg_name: String, def_val: T) -> T where <T as FromStr>::Err: std::fmt::Debug {
    let param: String = env::args()
        .find_map(|arg| {
            if arg.contains(&arg_name) {
                return Some(arg.replace(&arg_name, ""));
            }

            None
        })
        .unwrap_or_else(|| "".to_string());

    return if param.parse::<T>().is_ok() { param.parse::<T>().unwrap() } else { def_val };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging.
    Builder::new().filter(None, LevelFilter::Info).target(Target::Stdout).init();

    info!("Speedometer mood lightning Consumer has started.");

    #[cfg(target_arch = "aarch64")]
    let mut display: led_driver::Display = Default::default();

    #[cfg(target_arch = "aarch64")]
    {
        const DEFAULT_LED_COUNT: i32 = 8;
        const DEFAULT_LED_DELAY_MS: u64 = 100;
        display.init();

        let led_count = get_cmd_arg(LED_COUNT_FLAG.to_string(), DEFAULT_LED_COUNT);
        let led_delay_ms = get_cmd_arg(LED_DELAY_MS_FLAG.to_string(), DEFAULT_LED_DELAY_MS);
        let led_color = get_cmd_arg(LED_COLOR_FLAG.to_string(), RED_RGB_COLOR);
        let bb_mode = &get_cmd_arg(BLUEBOLT_MODE.to_string(), BLUEBOLT_MODE_DEFAULT.to_string()) as &str;

        unsafe {
            blink_delay = get_cmd_arg(BLINK_DELAY_FLAG.to_string(), BLINK_DEFAULT_DELAY);
            display_ptr = &mut display as *mut led_driver::Display;
            max_speed = get_cmd_arg(MAX_SPEED_FLAG.to_string(), MAX_SPEED_DEFAULT) as i32;
        }

        match bb_mode {
            BLUEBOLT_MODE_OFF => { display.setAllLedsToRgb(0x00000000); return Ok(());},
            BLUEBOLT_MODE_LED => led_driver::running_led(&mut display, led_color, led_delay_ms, led_count),
            BLUEBOLT_MODE_GRADIENT => led_driver::dynamic_gradinet(&mut display, 0x00200000, 0x00002000, led_delay_ms),
            BLUEBOLT_MODE_SPEED => info!("provider's speed"),
            _ => { led_driver::default_splash(&mut display); return Ok(());},
        }
    }

    let settings = consumer_config::load_settings();

    let invehicle_digital_twin_uri = retrieve_invehicle_digital_twin_uri(
        settings.invehicle_digital_twin_uri,
        settings.chariott_uri,
    )
    .await?;

    // Get subscription constraints.
    let default_frequency_ms: u64 = 300;
    let frequency_ms = env::args()
        .find_map(|arg| {
            if arg.contains(FREQUENCY_MS_FLAG) {
                return Some(arg.replace(FREQUENCY_MS_FLAG, ""));
            }

            None
        })
        .unwrap_or_else(|| default_frequency_ms.to_string());

    // Retrieve the provider URI.
    let provider_endpoint_info = discover_digital_twin_provider_using_ibeji(
        &invehicle_digital_twin_uri,
        sdv::vehicle_v3::vehicle_speed::ID,
        digital_twin_protocol::GRPC,
        &[digital_twin_operation::MANAGEDSUBSCRIBE.to_string()],
    )
    .await
    .unwrap();

    let managed_subscribe_uri = provider_endpoint_info.uri;
    info!("Speedometer mood lightning URI for the Vehicle speed property's provider is {managed_subscribe_uri}");

    // Create constraint for the managed subscribe call.
    let frequency_constraint = Constraint {
        r#type: constraint_type::FREQUENCY_MS.to_string(),
        value: frequency_ms.to_string(),
    };

    // Get the subscription information for a managed topic with constraints.
    let subscription_info = get_vehicle_subscription_info(
        &managed_subscribe_uri,
        vec![frequency_constraint],
    )
    .await?;

    // Deconstruct subscription information.
    let broker_uri = subscription_info.uri;
    let topic = subscription_info.context;
    info!("The broker URI for the Vehicle Speed property's provider is {broker_uri}");

    // Subscribe to topic.
    let sub_handle = receive_vehicle_data_updates(&broker_uri, &topic)
        .await
        .map_err(|err| Status::internal(format!("{err:?}")))?;

    //update_dashboard
    let _ws_handle = update_dashboard()
        .await
        .map_err(|err| Status::internal(format!("{err:?}")))?;

    signal::ctrl_c().await?;

    info!("The Consumer has completed. Shutting down...");

    // Wait for subscriber task to cleanly shutdown.
    _ = sub_handle.await;

    std::process::exit(0);

    Ok(())
}
