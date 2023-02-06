// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT license.

const MAX_HVAC_TEMPERATURE: u32 = 100;
const MIN_HVAC_TEMPERATURE: u32 = 65;

#[derive(Debug, Default)]
pub struct Vehicle {
    pub ambient_air_temperature: u32,
    pub is_air_conditioning_active: bool,
    pub ui_message: String,
    pub hybrid_battery_remaining: f32,
}

impl Vehicle {
    pub fn new() -> Self {
        Self {
            ambient_air_temperature: 75,
            is_air_conditioning_active: false,
            ui_message: String::new(),
            hybrid_battery_remaining: 100.0,
        }
    }

    pub fn execute_epoch(&mut self) {
        // A/C will not be active without power.
        if self.hybrid_battery_remaining == 0.0 {
            self.is_air_conditioning_active = false;
        }

        // Adjust the ambient air temperature based on whether the A/C is being used.
        // When the A/C is being used, then the temperature should decrease until it reaches the lower limit and
        // remain at the lower limit once it reaches it.
        // When the A/C is not being used, then the temperature should increase until it reaches the upper limit and
        // remain at the upper limit once it reaches it.
        if self.is_air_conditioning_active {
            if self.ambient_air_temperature > MIN_HVAC_TEMPERATURE {
                self.ambient_air_temperature -= 1;
            }
        } else if self.ambient_air_temperature < MAX_HVAC_TEMPERATURE {
            self.ambient_air_temperature += 1;
        }

        // Update the A/C's use of the battery.
        if self.is_air_conditioning_active && self.hybrid_battery_remaining > 0.0 {
            self.hybrid_battery_remaining -= 0.10;
            // To deal with rounding errors, once we are below 0.10, we will set hybrid_battery_remaining to 0.0
            if self.hybrid_battery_remaining < 0.10 {
                self.hybrid_battery_remaining = 0.0
            }
        }
    }
}
