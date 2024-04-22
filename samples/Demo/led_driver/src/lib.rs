#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn init(ledCount: usize) -> ws2811_t {
    let mut panel = ws2811_t {
        render_wait_time: 0,
        device: std::ptr::null_mut(),
        rpi_hw: std::ptr::null_mut(),
        freq: WS2811_TARGET_FREQ,
        dmanum: 10,
        channel: [
            ws2811_channel_t {
                gpionum: 18,
                invert: 0,
                count: ledCount as i32,
                strip_type: 0x00081000,
                leds: std::ptr::null_mut(),
                brightness: 255,
                wshift: 0,
                rshift: 0,
                gshift: 0,
                bshift: 0,
                gamma: std::ptr::null_mut(),
            },
            ws2811_channel_t {
                gpionum: 0,
                invert: 0,
                count: 0,
                strip_type: 0,
                leds: std::ptr::null_mut(),
                brightness: 0,
                wshift: 0,
                rshift: 0,
                gshift: 0,
                bshift: 0,
                gamma: std::ptr::null_mut(),
            },
        ],
    };
    unsafe {
        let ptr = &mut panel as *mut ws2811_t;
        ws2811_init(ptr);
    }

    return panel;
}

pub fn setAllLedsToRgb(panel: &mut ws2811_t, colorCodeRgb: u32) {
    println!("setAllLedsToRgb: colorCodeRgb = {:06x}", colorCodeRgb);

    unsafe {
        for i in 0..=panel.channel[0].count-1 {
            std::ptr::write(panel.channel[0].leds.add(i as usize), colorCodeRgb);
        }
        let ptr = panel as *mut ws2811_t;
        ws2811_render(ptr);
    }
}
