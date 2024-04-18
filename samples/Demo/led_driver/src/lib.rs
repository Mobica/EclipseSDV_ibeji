#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn init() -> ws2811_t {
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
                count: 8,
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

pub fn allGreen(ledString: &mut ws2811_t) {
    println!("called allGreen");

    unsafe {
        std::ptr::write(ledString.channel[0].leds, 0x00002000);
        //ledString.channel[0].leds.wrapping_add(0) = 0x00002000;
        let ptr = ledString as *mut ws2811_t;
        ws2811_render(ptr);
    }
}

pub fn allRed(ledString: &mut ws2811_t) {
    println!("called allRed");

    unsafe {
        std::ptr::write(ledString.channel[0].leds, 0x00200000);
        let ptr = ledString as *mut ws2811_t;
        ws2811_render(ptr);
    }
}
