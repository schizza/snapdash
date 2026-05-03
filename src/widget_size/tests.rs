use crate::helpers::humanize_magnitude;

#[test]
fn compresses_large_watts() {
    assert_eq!(humanize_magnitude("1234567 W"), "1.23 MW");
    assert_eq!(humanize_magnitude("5000 W"), "5.00 kW");
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
    assert_eq!(humanize_magnitude("0.0005 A"), "500.00 µA");
}
