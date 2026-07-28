use crate::helpers::humanize_magnitude;
use crate::widget_size::WidgetSize;

/// An entity with no axis has no controls to reveal, so there is nothing
/// to grow into and the widget stays at its preset.
#[test]
fn an_entity_with_no_axes_grows_by_nothing() {
    for &size in WidgetSize::ALL {
        assert_eq!(size.controls_height(0), 0.0, "{size}");
    }
}

/// `entity_window` puts a separating gap in front of every control row,
/// so the height each axis costs is the gap plus the row. A widget that
/// counted one gap for the whole block came up short as soon as it had a
/// second axis, and the last slider paid for it.
#[test]
fn every_axis_costs_its_own_gap() {
    for &size in WidgetSize::ALL {
        let one = size.controls_height(1);
        assert_eq!(
            one,
            size.value_detail_gap() + size.control_row_height(),
            "{size}"
        );
        assert_eq!(size.controls_height(2), one * 2.0, "{size}");
        assert_eq!(size.controls_height(3), one * 3.0, "{size}");
    }
}

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

#[test]
fn scales_into_nano_and_pico() {
    assert_eq!(humanize_magnitude("0.000000001 A"), "1 nA"); // 1e-9
    assert_eq!(humanize_magnitude("0.0000000005 A"), "500 pA"); // 5e-10
    assert_eq!(humanize_magnitude("0.000000000001 A"), "1 pA"); // 1e-12
}

#[test]
fn boundary_between_milli_and_micro() {
    assert_eq!(humanize_magnitude("0.001 A"), "1 mA");
    assert_eq!(humanize_magnitude("0.0001 A"), "100 µA"); // dropped from milli to micro
}
