use colorsys::Hsl;
use tapo::requests::Color;
use crate::tapo::server::rpc::Rgb;

/// Convert either a kelvin temperature or a hsl value to a rgb value
pub fn any_to_rgb(temperature: Option<u32>, hue: Option<u32>, saturation: Option<u32>, brightness: Option<u32>) -> Option<Rgb> {
    if let Some((hue, saturation, brightness)) = hue.zip(saturation).zip(brightness).map(|((a, b), c)| (a, b, c)) {
        let hsl = Hsl::new(hue as f64, saturation as f64, brightness as f64, None);
        let rgb = colorsys::Rgb::from(hsl);
        Some(Rgb { red: rgb.red().round() as u32, blue: rgb.blue().round() as u32, green: rgb.green().round() as u32 })
    } else { temperature.map(kelvin_to_rgb) }
}

/// Convert a kelvin temperature value to an approximated rgb value
///
/// https://github.com/spacekookie/colortemp/blob/ed421d6e928d4ed394be241f511661d588142766/src/lib.rs#L51
fn kelvin_to_rgb(temperature: u32) -> Rgb {
    let (mut r, mut g, mut b);
    let temp = temperature / 100;

    if temp <= 66 {
        r = 255f64;
    } else {
        r = (temp as f64) - 60f64;
        r = 329.698727446 * r.powf(-329.698727446);
        r = f64::max(f64::min(r, 255f64), 0f64);
    }

    if temp <= 66 {
        g = temp as f64;
        g = 99.4708025861 * g.ln() - 161.1195681661;
        g = f64::max(f64::min(g, 255f64), 0f64);
    } else {
        g = temp as f64 - 60f64;
        g = 288.1221695283 * g.powf(-0.0755148492);
        g = f64::max(f64::min(g, 255f64), 0f64);
    }

    if temp >= 66 {
        b = 255f64;
    } else if temp <= 19 {
        b = 0f64;
    } else {
        b = temp as f64 - 10f64;
        b = 138.5177312231 * b.ln() - 305.0447927307;
        b = f64::max(f64::min(b, 255f64), 0f64);
    }

    Rgb {
        red: r.round() as u32,
        green: g.round() as u32,
        blue: b.round() as u32,
    }
}

/// Convert a google home color to it's hue, saturation and temperature value
pub fn color_to_hst(color: Color) -> (u32, u32, u32) {
    match color {
        Color::CoolWhite => (0, 100, 4000),
        Color::Daylight => (0, 100, 5000),
        Color::Ivory => (0, 100, 6000),
        Color::WarmWhite => (0, 100, 3000),
        Color::Incandescent => (0, 100, 2700),
        Color::Candlelight => (0, 100, 2500),
        Color::Snow => (0, 100, 6500),
        Color::GhostWhite => (0, 100, 6500),
        Color::AliceBlue => (208, 5, 0),
        Color::LightGoldenrod => (54, 28, 0),
        Color::LemonChiffon => (54, 19, 0),
        Color::AntiqueWhite => (0, 100, 5500),
        Color::Gold => (50, 100, 0),
        Color::Peru => (29, 69, 0),
        Color::Chocolate => (30, 100, 0),
        Color::SandyBrown => (27, 60, 0),
        Color::Coral => (16, 68, 0),
        Color::Pumpkin => (24, 90, 0),
        Color::Tomato => (9, 72, 0),
        Color::Vermilion => (4, 77, 0),
        Color::OrangeRed => (16, 100, 0),
        Color::Pink => (349, 24, 0),
        Color::Crimson => (348, 90, 0),
        Color::DarkRed => (0, 100, 0),
        Color::HotPink => (330, 58, 0),
        Color::Smitten => (329, 67, 0),
        Color::MediumPurple => (259, 48, 0),
        Color::BlueViolet => (271, 80, 0),
        Color::Indigo => (274, 100, 0),
        Color::LightSkyBlue => (202, 46, 0),
        Color::CornflowerBlue => (218, 57, 0),
        Color::Ultramarine => (254, 100, 0),
        Color::DeepSkyBlue => (195, 100, 0),
        Color::Azure => (210, 100, 0),
        Color::NavyBlue => (240, 100, 0),
        Color::LightTurquoise => (180, 26, 0),
        Color::Aquamarine => (159, 50, 0),
        Color::Turquoise => (174, 71, 0),
        Color::LightGreen => (120, 39, 0),
        Color::Lime => (75, 100, 0),
        Color::ForestGreen => (120, 75, 0),
    }
}