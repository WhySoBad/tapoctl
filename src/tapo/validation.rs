use validator::ValidationError;

use super::server::rpc::{HueSaturation, IntegerValueChange};

/// Validate the hue range of an integer change
pub fn validate_hue(change: &IntegerValueChange) -> Result<(), ValidationError> {
    if change.absolute && !(1..=360).contains(&change.value) {
        Err(ValidationError::new(
            "Hue value has to be in range 1 to 360",
        ))
    } else {
        Ok(())
    }
}

/// Validate the temperature range of an integer change
pub fn validate_temperature(change: &IntegerValueChange) -> Result<(), ValidationError> {
    if change.absolute && !(2500..=6500).contains(&change.value) {
        Err(ValidationError::new(
            "Temperature value has to be in range 2500 to 6500",
        ))
    } else {
        Ok(())
    }
}

/// Validate the saturation range of an integer change
pub fn validate_saturation(change: &IntegerValueChange) -> Result<(), ValidationError> {
    if change.absolute && !(1..=100).contains(&change.value) {
        Err(ValidationError::new(
            "Saturation value has to be in range 1 to 100",
        ))
    } else {
        Ok(())
    }
}

/// Validate the brightness range of an integer change
pub fn validate_brightness(change: &IntegerValueChange) -> Result<(), ValidationError> {
    if change.absolute && !(1..=100).contains(&change.value) {
        Err(ValidationError::new(
            "Brightness value has to be in range 1 to 100",
        ))
    } else {
        Ok(())
    }
}

/// Validate the hue and saturation ranges of a hue and/or saturation change
pub fn validate_hue_saturation(hs: &HueSaturation) -> Result<(), ValidationError> {
    if hs.hue.is_some() && hs.saturation.is_none() {
        Err(ValidationError::new(
            "Saturation has to be set as well if hue is set",
        ))?
    } else if hs.hue.is_none() && hs.saturation.is_some() {
        Err(ValidationError::new(
            "Hue has to be set as well if saturation is set",
        ))?
    }

    if let Some(hue) = &hs.hue {
        validate_hue(hue)?;
    }
    if let Some(saturation) = &hs.saturation {
        validate_saturation(saturation)?;
    }
    Ok(())
}
