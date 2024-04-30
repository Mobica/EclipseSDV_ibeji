#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub struct Display {
    panel: ws2811_t,
}

impl Display {
    pub fn init(&mut self) {
        unsafe {
            let ptr = &mut self.panel as *mut ws2811_t;
            ws2811_init(ptr);
        }
    }

    pub fn setAllLedsToRgb(&mut self, colorCodeRgb: u32) {
        println!("setAllLedsToRgb: colorCodeRgb = {:06x}", colorCodeRgb);
    
        unsafe {
            for i in 0..=self.panel.channel[0].count-1 {
                std::ptr::write(self.panel.channel[0].leds.add(i as usize), colorCodeRgb);
            }
            let ptr = &mut self.panel as *mut ws2811_t;
            ws2811_render(ptr);
        }
    }

    pub fn setRgbGradient(&mut self, colorCodeRgbLeft: u32, colorCodeRgbRight: u32) {
        println!("setRgbGradient: colorCodeRgbLeft = {:06x}, colorCodeRgbRight = {:06x}", colorCodeRgbLeft, colorCodeRgbRight);
    
        self.setRgbGradientMod(colorCodeRgbLeft, colorCodeRgbRight, 0, 31)
    }

    pub fn setRgbGradientMod(&mut self, colorCodeRgbLeft: u32, colorCodeRgbRight: u32, start: usize, end: usize) {
        println!("setRgbGradient: colorCodeRgbLeft = {:06x}, colorCodeRgbRight = {:06x}, start: {start}, end: {end}", colorCodeRgbLeft, colorCodeRgbRight);
    
        let gradient = self.generateGradient32(colorCodeRgbLeft, colorCodeRgbRight, start, end);
    
        unsafe {
            for i in 0..=self.panel.channel[0].count-1 {
                std::ptr::write(self.panel.channel[0].leds.add(i as usize), gradient[i as usize / 8]);
            }
            let ptr = &mut self.panel as *mut ws2811_t;
            ws2811_render(ptr);
        }
    }
    
    pub fn setOnlyOneLedToRgb(&mut self, ledId: u32, colorCodeRgb: u32) {
        println!("setOnlyOneLedToRgb: id: {ledId}, colorCodeRgb = {:06x}", colorCodeRgb);
    
        unsafe {
            for i in 0..=self.panel.channel[0].count-1 {
                if ledId == i as u32 {
                    std::ptr::write(self.panel.channel[0].leds.add(i as usize), colorCodeRgb);
                } else {
                    std::ptr::write(self.panel.channel[0].leds.add(i as usize), 0x00000000);
                }
            }
            let ptr = &mut self.panel as *mut ws2811_t;
            ws2811_render(ptr);
        }
    }
    
    fn generateGradient8(&mut self, fromColor: u8, toColor: u8, start: usize, end: usize) -> [u8; 32] {

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
    
    fn generateGradient32(&mut self, fromColor: u32, toColor: u32, start: usize, end: usize) -> [u32; 32] {
    
        let mut array: [u32; 32] = Default::default();
        let decomposedFrom = fromColor.to_ne_bytes();
        let decomposedTo = toColor.to_ne_bytes();
        let gradientR = self.generateGradient8(decomposedFrom[2], decomposedTo[2], start, end);
        let gradientG = self.generateGradient8(decomposedFrom[1], decomposedTo[1], start, end);
        let gradientB = self.generateGradient8(decomposedFrom[0], decomposedTo[0], start, end);
    
        for i in 0..32 {
            array[i] = u32::from_ne_bytes([gradientB[i], gradientG[i], gradientR[i], 0x00]);
        }
    
        return array;
    }
    
    
}

impl Default for Display {
    fn default() -> Self {
        Self {
            panel: ws2811_t {
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
            }
        }
    }
}

pub fn running_led(display: &mut Display, led_color: u32, delay_ms: u64, max_count: i32)
{
    let count = if max_count < display.panel.channel[0].count { max_count } else { display.panel.channel[0].count };
    loop {
        for i in 0..=count-2 {
            display.setOnlyOneLedToRgb(i as u32, led_color);
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
        for i in 0..=count-2 {
            let id: u32 = (count -1 -i) as u32;
            display.setOnlyOneLedToRgb(id, led_color);
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }
}

pub fn dynamic_gradinet(display: &mut Display, color_code_rgb_left: u32, color_code_rgb_right: u32, led_delay_ms: u64)
{
    loop {
        for i in 0..31 {
            display.setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, 0, i);
            std::thread::sleep(std::time::Duration::from_millis(led_delay_ms));
        }
        for i in 0..31 {
            display.setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, i, 31);
            std::thread::sleep(std::time::Duration::from_millis(led_delay_ms));
        }
        for i in 0..31 {
            display.setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, 31-i, 31);
            std::thread::sleep(std::time::Duration::from_millis(led_delay_ms));
        }
        for i in 0..31 {
            display.setRgbGradientMod(color_code_rgb_left, color_code_rgb_right, 0, 31-i);
            std::thread::sleep(std::time::Duration::from_millis(led_delay_ms));
        }
    }
}

pub fn default_splash(display: &mut Display)
{
    display.setAllLedsToRgb(0x00200000);
    std::thread::sleep(std::time::Duration::from_secs(1));
    display.setRgbGradient(0x00200000, 0x00000020);
    std::thread::sleep(std::time::Duration::from_secs(1));
    display.setRgbGradientMod(0x00200000, 0x00000020, 10, 30);
    std::thread::sleep(std::time::Duration::from_secs(1));
}
