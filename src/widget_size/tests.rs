use crate::helpers::humanize_magnitude;

#[test]
fn compresses_large_watts() {
    assert_eq!(humanize_magnitude("1234567 W"), "1.23 MW");
    assert_eq!(humanize_magnitude("5000 W"), "5 kW");
}

#[test]
fn skips_in_range() {
    assert_eq!(humanize_magnitude("500 W"), "500 W");
    assert_eq!(humanize_magnitude("23 V"), "23 V");
}

#[test]
fn skips_unsupported_units() {
    assert_eq!(humanize_magnitude("23.5 °C"), "23.5 °C");
    assert_eq!(humanize_magnitude("45.2 %"), "45.2 %");
}

#[test]
fn skips_already_prefixed() {
    assert_eq!(humanize_magnitude("1.5 kW"), "1.5 kW");
}

#[test]
fn handles_non_numeric() {
    assert_eq!(humanize_magnitude("On"), "On");
    assert_eq!(humanize_magnitude(""), "");
}

#[test]
fn compresses_small() {
    assert_eq!(humanize_magnitude("0.0005 A"), "500 µA");
}
#[test]
fn does_not_scale_moderately_small_values() {
    assert_eq!(humanize_magnitude("0.1 A"), "0.1 A");
    assert_eq!(humanize_magnitude("0.5 A"), "0.5 A");
    assert_eq!(humanize_magnitude("0.05 A"), "0.05 A");
}

#[test]
fn scales_truly_small_values() {
    assert_eq!(humanize_magnitude("0.005 A"), "5 mA");
    assert_eq!(humanize_magnitude("0.0001 A"), "100 µA");
    assert_eq!(humanize_magnitude("0.0051 A"), "5.1 mA");
}

#[test]
fn trims_trailing_zeros() {
    assert_eq!(humanize_magnitude("5000 W"), "5 kW");
    assert_eq!(humanize_magnitude("1500 W"), "1.5 kW");
    assert_eq!(humanize_magnitude("1230000 W"), "1.23 MW");
}
