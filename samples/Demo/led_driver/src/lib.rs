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
                gpionum: 21,
                invert: 0,
                count: 256,
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

pub fn setRgbGradient(panel: &mut ws2811_t, colorCodeRgbLeft: u32, colorCodeRgbRight: u32) {
    println!("setRgbGradient: colorCodeRgbLeft = {:06x}, colorCodeRgbRight = {:06x}", colorCodeRgbLeft, colorCodeRgbRight);

    setRgbGradientMod(panel, colorCodeRgbLeft, colorCodeRgbRight, 0, 31)
}

fn generateGradient8(fromColor: u8, toColor: u8, start: usize, end: usize) -> [u8; 32] {

    let divider = if end <= start {32.0} else {(end - start) as f64};
    let step = if fromColor > toColor {(fromColor - toColor) as f64 / divider} else {(toColor - fromColor) as f64 / divider};
    let mut array: [u8; 32] = Default::default();

    for i in 0..start {
        array[i] = fromColor;
    }
    for i in end..32 {
        array[i] = toColor;
    }
    for i in 0..=end-start {
        array[start + i] = if fromColor > toColor {fromColor - ((i as f64 * step) as u8) & 0xff} 
                                     else {(fromColor + (i as f64 * step) as u8) & 0xff};
    }

    return array;
}

fn generateGradient32(fromColor: u32, toColor: u32, start: usize, end: usize) -> [u32; 32] {

    let mut array: [u32; 32] = Default::default();
    let decomposedFrom = fromColor.to_ne_bytes();
    let decomposedTo = toColor.to_ne_bytes();
    let gradientR = generateGradient8(decomposedFrom[2], decomposedTo[2], start, end);
    let gradientG = generateGradient8(decomposedFrom[1], decomposedTo[1], start, end);
    let gradientB = generateGradient8(decomposedFrom[0], decomposedTo[0], start, end);

    for i in 0..32 {
        array[i] = u32::from_ne_bytes([gradientB[i], gradientG[i], gradientR[i], 0x00]);
    }

    return array;
}

pub fn setRgbGradientMod(panel: &mut ws2811_t, colorCodeRgbLeft: u32, colorCodeRgbRight: u32, start: usize, end: usize) {
    println!("setRgbGradient: colorCodeRgbLeft = {:06x}, colorCodeRgbRight = {:06x}, start: {start}, end: {end}", colorCodeRgbLeft, colorCodeRgbRight);

    let gradient = generateGradient32(colorCodeRgbLeft, colorCodeRgbRight, start, end);

    unsafe {
        for i in 0..=panel.channel[0].count-1 {
            std::ptr::write(panel.channel[0].leds.add(i as usize), gradient[i as usize / 8]);
        }
        let ptr = panel as *mut ws2811_t;
        ws2811_render(ptr);
    }
}

pub fn setOnlyOneLedToRgb(panel: &mut ws2811_t, ledId: u32, colorCodeRgb: u32) {
    println!("setOnlyOneLedToRgb: id: {ledId}, colorCodeRgb = {:06x}", colorCodeRgb);

    unsafe {
        for i in 0..=panel.channel[0].count-1 {
            if ledId == i as u32 {
                std::ptr::write(panel.channel[0].leds.add(i as usize), colorCodeRgb);
            } else {
                std::ptr::write(panel.channel[0].leds.add(i as usize), 0x00000000);
            }
        }
        let ptr = panel as *mut ws2811_t;
        ws2811_render(ptr);
    }
}

