#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn init(ledString: &mut ws2811_t) {
    ledString.freq = WS2811_TARGET_FREQ;
    ledString.dmanum = 10;
    ledString.channel[0].gpionum = 18;
    ledString.channel[0].count = 8;
    ledString.channel[0].strip_type = 0x00081000;
    ledString.channel[0].brightness = 255;

    unsafe {
        let ptr = ledString as *mut ws2811_t;
        ws2811_init(ptr);
    }
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
